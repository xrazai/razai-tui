# Banco local

Este projeto esta preparado para usar PostgreSQL local via Docker.

## Subir o banco

Instale o Docker Desktop e, depois de abrir o Docker, rode:

```powershell
docker compose up -d
```

## Parar o banco

```powershell
docker compose down
```

## Variaveis locais

Copie `.env.example` para `.env` e mantenha o `.env` fora do Git.

O valor de `DATABASE_URL` no exemplo aponta para o Postgres local criado pelo `docker-compose.yml`. Se voce trocar usuario, senha, porta ou nome do banco no Docker, atualize tambem o `.env` local.

Nao documente chaves reais de API ou senhas pessoais no README, docs ou commits. `OPENROUTER_API_KEY` e credenciais `SHOPEE_*` devem existir apenas no `.env` local.

## Dados

As migrations ficam em `migrations/`. Na primeira vez que o container sobe, o Postgres executa os arquivos `.sql` dessa pasta.

Tabelas principais:

- `tecidos`: tecidos cadastrados, SKU, composicao, largura, custo base, precos de venda, tipo, gramaturas e fornecedor opcional.
- `cores`: cores cadastradas, hexadecimal, swatch derivado e SKU.
- `estampas`: estampas cadastradas e SKU.
- `fornecedores`: fornecedores cadastrados para associar a tecidos e relatorios.
- `tecido_cores`: vinculos de tecidos lisos com cores.
- `tecido_estampas`: vinculos de tecidos estampados com estampas.
- `vendas` e `venda_itens`: historico de vendas e itens, incluindo identidade de estoque do vinculo vendido.
- `pedidos` e `pedido_itens`: pedidos pendentes/aprovados, PDF salvo e itens com identidade de estoque.
- `estoque_movimentacoes`: fonte de verdade do saldo de estoque por vinculo.
- `estoque_ordens`: pendencias automaticas de falta de estoque para direcionamento a fornecedor.
- `shopee_stock_policies`: alvos persistidos de estoque online por item/modelo Shopee.
- `configuracoes`: configuracoes locais persistidas no banco, como impressora de recibos e limiar Delta E de cores.

O app tambem garante em runtime as tabelas e colunas recentes, como `fornecedores`, `configuracoes`, `estampas`, `tecido_estampas`, `estoque_movimentacoes`, `estoque_ordens`, `shopee_stock_policies`, `tecidos.fornecedor_id`, `tecidos.custo_base`, `tecidos.preco_atacado`, `tecidos.preco_varejo`, `custo_override` e overrides de preco nos vinculos, porque bancos locais antigos podem ter sido criados antes dessas migrations.

## Estoque

`estoque_movimentacoes` registra todas as entradas e saidas:

- `entrada`: quantidade positiva criada manualmente.
- `saida_venda`: quantidade negativa criada ao salvar venda ou aprovar pedido.
- `saida_transferencia`: quantidade negativa criada manualmente para transferencia.

O saldo nao fica duplicado em tabela fixa; ele e calculado por `SUM(quantidade)` agrupado por `tecido_id`, `item_id` e `usa_estampas`.

`estoque_ordens` e uma pendencia operacional, nao uma movimentacao. Ela e criada quando uma saida de venda excede o saldo disponivel antes da baixa. Campos principais: vinculo, quantidade faltante, status (`pendente`, `direcionada`, `concluida`, `cancelada`), fornecedor opcional e `venda_id` de origem. Editar ou excluir venda remove/recalcula as ordens automaticas daquela venda.

Relatorios:

- `Resumo fornecedor`: filtra vendas por `tecidos.fornecedor_id` e periodo, somando quantidade vendida e custo vendido.
- `Mais vendidos`: agrupa vendas por vinculo vendido e ordena por quantidade.

## Imagens de Vinculos

As tabelas `tecido_cores` e `tecido_estampas` possuem quatro colunas `BYTEA` para imagens do vinculo:

| Coluna | Uso |
| --- | --- |
| `imagem_original` | Foto principal/original do vinculo. |
| `imagem_brand` | Imagem de marca/branding. |
| `imagem_modelo` | Imagem com modelo. |
| `imagem_alternativa` | Imagem alternativa/complementar. |
| `custo_override` | Custo especifico do vinculo quando uma cor/estampa foge do custo base do tecido. Vazio usa `tecidos.custo_base`. |
| `preco_atacado_override` | Preco de venda de atacado especifico do vinculo. Vazio usa `tecidos.preco_atacado`. |
| `preco_varejo_override` | Preco de venda de varejo especifico do vinculo. Vazio usa `tecidos.preco_varejo`. |
| `ativo` | Controla se o vinculo aparece para novos lancamentos. `Desfazer Vinculo` marca `ativo = false` sem remover o registro nem suas imagens. |

Ao salvar novamente a lista de vinculos de um tecido, os registros mantidos preservam essas imagens; vinculos desmarcados sao marcados como inativos. Se forem selecionados novamente, sao reativados.

O TUI pode renderizar thumbnail de qualquer um dos quatro slots no detalhe do vinculo. As imagens continuam armazenadas como bytes originais no banco; cache, redimensionamento e protocolo de terminal sao apenas estado de exibicao em memoria.

## Configuracoes

Configuracoes usam pares `chave`/`valor`.

| Chave | Uso |
| --- | --- |
| `receipt_printer` | Nome da impressora de recibos 80mm selecionada em `Configuracoes`. |
| `color_delta_e_threshold` | Limiar CIEDE2000 usado para bloquear cores visualmente proximas. Padrao: `3`. |
| `shopee_access_token` | Token de acesso Shopee vigente. |
| `shopee_refresh_token` | Refresh token Shopee vigente. |
| `shopee_access_token_expires_at` | Expiracao UNIX do access token. |
| `shopee_refresh_token_expires_at` | Expiracao UNIX estimada do refresh token. |

## Shopee

A fonte preferencial dos tokens Shopee e a tabela `configuracoes`. O `.env` funciona como seed inicial e espelho local legivel.

No startup e antes de chamadas Shopee, o app:

- carrega tokens do banco;
- usa `.env` se o banco ainda nao tiver tokens;
- renova o access token quando estiver vencido ou perto de vencer;
- persiste tokens renovados no banco e no `.env`.

Valores reais de `SHOPEE_PARTNER_KEY`, tokens e authtokens de tunel nunca devem ser commitados.

`shopee_stock_policies` guarda a politica local para manter estoque online em um alvo:

| Coluna | Uso |
| --- | --- |
| `item_id`, `model_id` | Identidade remota da variacao. Itens sem variacao usam `model_id = 0`. |
| `sku`, `parent_sku` | SKUs normalizados para exibicao e reconciliacao. |
| `target_stock` | Alvo local salvo pela tela Shopee, normalmente `0` ou `100`. |
| `enabled` | Define se a politica participa da reconciliacao. |
| `last_remote_stock` | Ultimo estoque remoto observado pelo app. |
| `last_synced_at` | Ultimo sync/reconciliacao bem-sucedido. |
| `last_error` | Ultimo erro de sync, quando houver. |

A reconciliacao consulta a Shopee antes de chamar `product/update_stock`; o webhook `/shopee/push` apenas dispara essa reconciliacao. Isso evita confiar em payload de push como fonte de estoque.

Se precisar recriar o banco do zero:

```powershell
docker compose down -v
docker compose up -d
```

O `-v` apaga o volume de dados local.
