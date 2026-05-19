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

- `tecidos`: tecidos cadastrados, SKU, composicao, largura, tipo e gramaturas.
- `cores`: cores cadastradas, hexadecimal, swatch derivado e SKU.
- `estampas`: estampas cadastradas e SKU.
- `tecido_cores`: vinculos de tecidos lisos com cores.
- `tecido_estampas`: vinculos de tecidos estampados com estampas.
- `configuracoes`: configuracoes locais persistidas no banco, como impressora de recibos e limiar Delta E de cores.

O app tambem garante em runtime as tabelas `configuracoes`, `estampas` e `tecido_estampas`, porque bancos locais antigos podem ter sido criados antes dessas migrations.

## Imagens de Vinculos

As tabelas `tecido_cores` e `tecido_estampas` possuem quatro colunas `BYTEA` para imagens do vinculo:

| Coluna | Uso |
| --- | --- |
| `imagem_original` | Foto principal/original do vinculo, usada para thumbnail no TUI. |
| `imagem_brand` | Imagem de marca/branding. |
| `imagem_modelo` | Imagem com modelo. |
| `imagem_alternativa` | Imagem alternativa/complementar. |

Ao salvar novamente a lista de vinculos de um tecido, os registros mantidos preservam essas imagens; apenas os vinculos desmarcados sao removidos.

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

Se precisar recriar o banco do zero:

```powershell
docker compose down -v
docker compose up -d
```

O `-v` apaga o volume de dados local.
