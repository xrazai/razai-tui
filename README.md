# Razai TUI

Sistema de loja em terminal feito em Rust com `ratatui`.

## Rodar

Suba o Postgres local:

```powershell
docker compose up -d
```

Rode o app:

```powershell
cargo run
```

Para hot reload durante desenvolvimento:

```powershell
cargo watch -x run
```

## Configuracao local

Copie o arquivo de exemplo e preencha apenas o que for necessario:

```powershell
Copy-Item .env.example .env
```

Notas:

- `.env` fica fora do Git e deve guardar valores locais/sensiveis.
- `DATABASE_URL` ja vem configurada no `.env.example` para o Postgres do `docker-compose.yml`.
- `OPENROUTER_API_KEY` e opcional; sem ela o chat usa respostas locais limitadas.
- As variaveis `SHOPEE_*` habilitam a integracao Shopee. Tokens e chaves reais devem ficar apenas no `.env`.
- Nunca coloque chaves reais, tokens ou senhas pessoais no README ou em arquivos versionados.

## Navegacao

- `Ctrl+C`: sair
- `Esc`: voltar/cancelar
- `Tab`/`Shift+Tab`: alternar foco entre sistema, resumo quando visivel, e chat
- `Esq`/`Dir`: navegar entre abas
- `Cima`/`Baixo`: navegar em listas e campos
- `Enter`: abrir, avancar ou confirmar acao selecionada
- `Space`: marcar/desmarcar itens em vinculos e marcar impressora
- `Backspace`: apagar texto em campos editaveis

## Abas

- `Dashboard`: agente mestre para consultas e acoes com confirmacao.
- `Vendas`: nova venda, historico, edicao e exclusao.
- `Pedidos`: novo pedido, historico, PDF, compartilhamento nativo do Windows e aprovacao para virar venda.
- `Dados`: cadastros e vinculos.
- `Estoque`: reservado para estoque.
- `Shopee`: conexao Shopee, criacao de anuncio, estoque online por SKU e guia BR.
- `Documentos`: emissao de documentos operacionais, como checklist de vinculos em PDF.
- `Configuracoes`: impressora de recibos.

## Dados

`Dados` possui quatro fluxos:

- `Tecido`: cadastro e edicao de tecidos.
- `Cores`: cadastro e edicao de cores com hexadecimal, swatch e SKU automatico.
- `Estampas`: cadastro e edicao de estampas com SKU automatico.
- `Vinculos`: vincula tecidos a cores ou estampas.

Regra de vinculos:

- Tecido `Liso` usa cores cadastradas.
- Tecido `Estampado` usa estampas cadastradas.

Na lista de vinculos, `Enter` abre o detalhe do vinculo selecionado. Cada vinculo aceita quatro imagens salvas no banco local:

- `Imagem Original`
- `Imagem Brand`
- `Imagem Modelo`
- `Imagem Alternativa`

No detalhe do vinculo, selecione o slot com `Cima/Baixo` e pressione `Enter` para abrir a janela nativa do Windows e escolher a imagem no computador. A `Imagem Original` e renderizada como thumbnail no terminal via `ratatui-image` quando o terminal/backend permitir; caso contrario o sistema mantém a imagem salva e informa que nao conseguiu gerar preview.

## Vendas

O fluxo de nova venda e:

1. Selecionar tecido.
2. O sistema carrega automaticamente os vinculos corretos:
   - cores para tecido `Liso`
   - estampas para tecido `Estampado`
3. Selecionar o vinculo.
4. Informar preco unitario e quantidade.
5. Conferir o resumo do pedido.
6. Finalizar, ou finalizar e imprimir recibo direto na impressora configurada.

O `Resumo do pedido` aparece apenas na tela de lancamento, inclusive quando uma venda do historico esta aberta para edicao. Use `Tab` para focar o resumo, `Cima/Baixo` para selecionar um lancamento, `Enter` para editar e `Delete` para excluir com confirmacao.

No `Historico de Vendas`, o periodo padrao e o dia atual. Ajuste `Data inicio` e `Data fim` no formato `AAAA-MM-DD` e pressione `Enter` para recarregar. Na lista, `Enter` abre a venda selecionada para edicao. A tela de edicao permite salvar alteracoes, salvar e imprimir, cancelar ou excluir com confirmacao.

## Pedidos

Pedidos usam o mesmo fluxo de lancamento de vendas, mas geram uma pendencia em vez de uma venda imediata:

1. Selecionar tecido e vinculo.
2. Informar preco unitario e quantidade.
3. Gerar pedido.
4. O sistema salva o pedido como `pendente`, gera um PDF em `pdf_pedidos/` e abre o compartilhamento nativo do Windows com o PDF anexado.
5. Depois do pagamento, abra o pedido no historico e aprove para converter em venda.

## Documentos

A aba `Documentos` fica antes de `Configuracoes` e possui:

1. `Imprimir Checklist`

O checklist permite marcar um ou mais tecidos com `Space` e gerar o PDF com `Ctrl+Enter` ou pela opcao `[Gerar PDF]`. O arquivo e salvo fora do workspace, em `Documents\Razai\checklists`, para nao reiniciar o app quando ele estiver rodando com `cargo watch`.

Depois de gerar, o sistema chama a acao de impressao do Windows para o PDF. Se o visualizador padrao nao oferecer impressao direta, o sistema tenta abrir o PDF como fallback.

O PDF separa uma tabela para cada tecido selecionado. Cada linha mostra:

- thumbnail da cor com aproximadamente `1,5cm x 1,5cm`;
- tecido;
- nome da cor;
- checkbox para conferencia manual.

O gerador evita quebrar uma tabela entre paginas quando a tabela inteira ainda cabe em uma nova pagina. Tecidos sem vinculos tambem aparecem no PDF com uma linha informativa.

## Shopee

A aba `Shopee` possui:

1. `Criar anuncio`
2. `Estoque Online`
3. `Guia Shopee BR`

### Conexao

No startup, o app inicia o callback local, tenta detectar/iniciar o ngrok e atualiza as URLs publicas no `.env`:

- `SHOPEE_REDIRECT_URL`: rota OAuth, normalmente `https://...ngrok.../shopee/callback`.
- `SHOPEE_PUSH_WEBHOOK_URL`: rota de push/webhook, normalmente `https://...ngrok.../shopee/push`.

Para conectar a loja, abra o link terminado em `/shopee/auth`. A rota `/shopee/callback` e apenas o retorno OAuth que a Shopee chama com `code`.

### Estoque Online

`Estoque Online` carrega os anuncios/modelos da Shopee, agrupa primeiro por SKU Pai (`item_sku`) e exibe as variacoes (`model_sku`) dentro de cada pai:

- SKU Pai;
- SKU da variacao;
- quantidade de ocorrencias;
- estoque atual somado;
- modo selecionado para sincronizacao.

Para acelerar lojas com muitos anuncios, os detalhes dos itens e as variacoes sao buscados em paralelo com concorrencia controlada.

Controles:

- `Enter`: se ainda nao carregou, busca o estoque; em SKU Pai, expande/recolhe; em variacao, confirma sync da variacao.
- `Cima/Baixo`: navega por pais e variacoes visiveis.
- `Space`: alterna a variacao selecionada entre `Zerar 0` e `Ativar 100`.
- `R`: limpa a lista carregada para recarregar no proximo `Enter`.

A sincronizacao altera apenas a variacao selecionada e exige confirmacao. Itens sem variacao usam `model_id=0`. Grupos multi-location ficam bloqueados para atualizacao automatica.

### Atualizar Anuncios

`Atualizar anuncios` seleciona um tecido local e busca anuncios Shopee com `item_sku` igual ao SKU do tecido. Para cada anuncio encontrado, o sistema valida a estrutura `Cor x Tamanho`, compara as cores publicadas com os vinculos locais e adiciona somente as cores faltantes.

Os novos modelos preservam os precos ja existentes por tamanho no proprio anuncio, entram com estoque inicial `1`, usam o SKU do vinculo e reaproveitam a imagem atual do tier de cor quando disponivel. O fluxo mostra uma previa e exige confirmacao antes de chamar a Shopee.

### Criar Anuncio

O fluxo de criacao seleciona um tecido local, usa a categoria `Roupas Femininas > Tecidos > Outros`, reaproveita a imagem de `SHOPEE_DEFAULT_IMAGE_PATH` ou a primeira imagem encontrada em `Pictures`, gera variacoes por cor e tamanho, calcula peso por gramatura linear, calcula preco por tamanho a partir do preco por metro e publica como `NORMAL`. Os requisitos detalhados ficam em:

- [docs/ShopeeDocs/SHOPEE_CRIAR_ANUNCIO_BR.md](docs/ShopeeDocs/SHOPEE_CRIAR_ANUNCIO_BR.md)
- [docs/ShopeeDocs/SHOPEE_ESTOQUE_SKU.md](docs/ShopeeDocs/SHOPEE_ESTOQUE_SKU.md)

## Agente IA

O chat lateral usa OpenRouter quando `OPENROUTER_API_KEY` esta configurada. O app usa um agente unico, o Razai Master, com capacidades para tecidos, cores, estampas, vinculos, vendas, pedidos, documentos, configuracoes, estoque e Shopee. A tela atual apenas define o contexto inicial do atendimento. Responda `sim` para confirmar uma acao pendente ou `nao` para cancelar. A matriz de capacidades fica em [docs/skills.md](docs/skills.md).

## Arquitetura

A organizacao atual evita arquivos grandes quando for possivel dividir:

- `src/main.rs`: bootstrap de banco, terminal e app.
- `src/app.rs`: estado principal, loop e roteamento.
- `src/app/`: handlers por dominio.
- `src/screens/`: renderizacao de telas.
- `src/models.rs` e `src/models/`: estado de formularios, enums e regras de SKU.
- `src/db.rs` e `src/db/`: acesso ao Postgres.
- `src/agent.rs`: contexto do Razai Master e chamada OpenRouter.
