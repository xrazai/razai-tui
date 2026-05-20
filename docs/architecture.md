# Arquitetura

O projeto e uma TUI em Rust com `ratatui`, banco local PostgreSQL e chat lateral via OpenRouter.

## Modulos

| Caminho | Responsabilidade |
| --- | --- |
| `src/main.rs` | Inicializa `.env`, banco, terminal e `App`. |
| `src/app.rs` | Estado principal, loop de eventos e roteamento de renderizacao. |
| `src/app/agent_actions.rs` | Acoes pendentes do agente mestre do Dashboard e execucao apos confirmacao. |
| `src/app/background.rs` | Slot tipado de tarefas em segundo plano usado por operacoes lentas da TUI. |
| `src/app/dados.rs` | Eventos e operacoes da aba Dados. |
| `src/app/vendas.rs` | Eventos e operacoes da aba Vendas. |
| `src/app/vendas/history.rs` | Filtro de periodo e abertura de vendas do historico. |
| `src/app/vendas/receipt.rs` | Montagem e envio RAW/ESC-POS de recibos. |
| `src/app/estoque.rs` | Eventos da aba Estoque, movimentacoes, ordens e relatorios. |
| `src/app/configuracoes.rs` | Eventos da aba Configuracoes e leitura de impressoras do Windows. |
| `src/app/pdf_actions.rs` | Acoes nativas do Windows para PDFs: compartilhamento, impressao, abertura e fallback. |
| `src/app/pedidos.rs` | Eventos da aba Pedidos, geracao de PDF e compartilhamento nativo do Windows. |
| `src/app/documentos.rs` | Eventos da aba Documentos e geracao do checklist de vinculos. |
| `src/app/documentos/pdf.rs` | Layout e escrita do PDF de checklist em `Documents\Razai\checklists`. |
| `src/shopee.rs` | Cliente Shopee, assinatura HMAC, OAuth/callback, refresh de tokens, estoque online e sync por SKU. |
| `src/screens/chrome.rs` | Header, tabs, footer e chat. |
| `src/screens/dados.rs` | Renderizacao de listas da aba Dados. |
| `src/screens/dados/forms.rs` | Renderizacao dos formularios de Dados. |
| `src/screens/vendas.rs` | Renderizacao do fluxo de Vendas. |
| `src/screens/estoque.rs` | Renderizacao de saldos, ordens e relatorios de estoque. |
| `src/screens/configuracoes.rs` | Renderizacao da selecao de impressora. |
| `src/screens/pedidos.rs` | Renderizacao do fluxo de Pedidos. |
| `src/screens/documentos.rs` | Renderizacao do menu Documentos e selecao de tecidos para checklist. |
| `src/screens/shopee.rs` | Renderizacao da aba Shopee, menu, grupos de estoque e status. |
| `src/models.rs` | Enums, formularios e regras de calculo. |
| `src/models/sku.rs` | Geracao de SKUs. |
| `src/db.rs` | Queries e comandos PostgreSQL gerais. |
| `src/db/sales.rs` | Persistencia de vendas, itens e historico. |
| `src/db/orders.rs` | Persistencia de pedidos pendentes, itens e aprovacao como venda. |
| `src/db/stock.rs` | Movimentacoes de estoque, saldos, ordens e relatorios. |
| `src/agent.rs` | Contexto do Razai Master e chamada OpenRouter. |

## Limite de tamanho

Evitar arquivos com mais de 600 linhas. Se passar disso e houver um corte claro por dominio, dividir em modulo novo. Manter junto apenas quando a logica for parte integral da mesma responsabilidade.

## Estado e renderizacao

`App` guarda o estado da aplicacao e delega:

- eventos para `src/app/*.rs`
- renderizacao para `src/screens/*.rs`
- persistencia para `src/db.rs`
- contexto de IA para `src/agent.rs`

## UX TUI

Acoes visuais entre colchetes, como `[Confirmar]`, `[Voltar]`, `[Gerar PDF]` e `[Desfazer Vinculo]`, devem ficar separadas do conteudo por uma linha vazia quando estiverem misturadas com dados de contexto. Essa linha vazia deve ser um item/linha proprio com mapeamento correto de selecao. Nao usar `\n` dentro do texto do item selecionavel, porque isso quebra o highlight do `ratatui`.

Listas longas devem usar scroll antecipado, mantendo algumas linhas de contexto abaixo do item selecionado antes de chegar ao limite inferior da area visivel.

Acoes destrutivas persistidas usam estilo vermelho e ficam sempre no fim do grupo de acoes. `Cancelar` fica reservado para abandonar edicao temporaria; quando a acao remove ou desativa dado persistido, o rotulo deve explicitar o efeito, como `[Excluir]`, `[Cancelar Pedido]` ou `[Desfazer Vinculo]`. Confirmacoes destrutivas usam modal com borda vermelha e rodape `[Enter/S] Confirmar   [Esc/N] Voltar`.

## Vinculos e Imagens

Vinculos de tecidos lisos ficam em `tecido_cores`; vinculos de tecidos estampados ficam em `tecido_estampas`. Cada vinculo pode armazenar quatro imagens em colunas `BYTEA`: `imagem_original`, `imagem_brand`, `imagem_modelo` e `imagem_alternativa`. O custo padrao vem de `tecidos.custo_base`; quando uma cor ou estampa tiver custo diferente, o vinculo grava `custo_override` e passa a usar esse valor efetivo. Precos de venda ficam separados: `tecidos.preco_atacado` e `tecidos.preco_varejo` sao os valores base por tecido, enquanto `preco_atacado_override` e `preco_varejo_override` nos vinculos registram excecoes.

O fluxo de detalhe do vinculo abre a janela nativa do Windows para selecionar uma imagem local para cada slot. O app valida se o arquivo e uma imagem suportada antes de gravar no banco e tenta detectar leituras incompletas, principalmente em arquivos vindos de Google Drive/Drives compartilhados ainda nao sincronizados.

A tela de detalhe de vinculo oculta o painel de agente/chat e usa a area principal inteira. Os atalhos `1` a `4` selecionam diretamente os slots de imagem, `Tab` avanca para o proximo vinculo e `Shift+Tab` volta para o anterior. Depois de salvar uma imagem, o app avanca para o proximo slot vazio; quando o vinculo fica completo, avanca para o proximo vinculo incompleto.

O detalhe tambem inclui a acao `[Desfazer Vinculo]`. A acao e um soft delete: marca o vinculo como inativo para novos lancamentos, mas preserva o registro e suas imagens no banco. Vendas e pedidos ja existentes permanecem como estavam porque seus itens armazenam a descricao textual do item.

A thumbnail no terminal usa `ratatui-image` com deteccao automatica de protocolo (`Sixel`, `Kitty`, `iTerm2` ou fallback `Halfblocks`). O protocolo pode ser forcado por `RAZAI_IMAGE_PROTOCOL=auto|sixel|kitty|iterm2|halfblocks`. A geracao do protocolo de imagem e cacheada em memoria por vinculo/slot e invalidada quando uma nova imagem e salva. A selecao nativa de arquivo e modal, mas a leitura/validacao/salvamento apos a escolha rodam em segundo plano e exibem indicador visual de salvamento.

Ao atualizar os vinculos de um tecido, o app preserva os registros existentes e marca itens desmarcados como inativos; isso evita perder imagens ja cadastradas e permite reativar um vinculo no futuro.

## Agente IA

O app usa um agente unico, o Razai Master. A tela atual define apenas o contexto/capacidade preferencial; o agente recebe contexto global de dados, vendas, pedidos, vinculos e configuracoes. Consultas locais podem ser respondidas diretamente. Acoes que gravam ou alteram dados ficam em `pending_agent_action` e so executam depois de confirmacao textual do usuario (`sim`/`nao`).

Acoes mapeadas inicialmente:

- cadastro de tecido, cor e estampa;
- criacao de vinculo entre tecido e cor/estampa;
- abertura de venda por id;
- filtro de historico por periodo;
- selecao de impressora.

## Banco local

O banco local usa Docker/PostgreSQL. As migrations ficam em `migrations/`.

O app tambem executa garantias de tabela para estruturas recentes, como `configuracoes` e `estampas`, para funcionar em bancos locais ja criados antes dessas migrations.

## Lista de Precos

`Dados > Lista de Precos` e o ponto operacional para valores do tecido:

- `Custo Base`: grava `tecidos.custo_base` e excecoes em `custo_override` nos vinculos.
- `Atacado`: grava `tecidos.preco_atacado` e excecoes em `preco_atacado_override`.
- `Varejo`: grava `tecidos.preco_varejo` e excecoes em `preco_varejo_override`.

O cadastro de tecido nao deve expor `custo_base` como campo principal. Se o usuario digitar em um vinculo o mesmo valor da base, o app remove o override e a origem volta a ser `base`.

A primeira lista de tecidos precisa mostrar quando a base esta vazia mas existem excecoes, incluindo contagem e faixa de menor/maior valor quando disponivel.

## Configuracoes

Configuracoes devem persistir no banco, na tabela `configuracoes`, com pares `chave`/`valor`.

Chaves atuais:

- `receipt_printer`: impressora selecionada para recibos de venda.
- `shopee_access_token`, `shopee_refresh_token`, `shopee_access_token_expires_at`, `shopee_refresh_token_expires_at`: tokens Shopee persistidos no banco e espelhados no `.env`.

## Vendas, Pedidos e Estoque

A aba `Configuracoes` lista impressoras instaladas no Windows com `Get-Printer`. A impressora selecionada e salva no banco.

Vendas finalizadas sao persistidas em `vendas` e `venda_itens`. O historico inicia filtrado pelo dia atual e permite ajustar `Data inicio` e `Data fim`. O `Resumo do pedido` so aparece no lancamento ou na edicao de uma venda aberta pelo historico; nele, lancamentos podem ser selecionados, editados individualmente ou excluidos com confirmacao. O envio de recibo 80mm usa impressao RAW/ESC-POS direto para a impressora configurada, sem abrir janela de impressao.

Itens de venda e pedido carregam tambem a identidade de estoque (`estoque_tecido_id`, `estoque_item_id`, `estoque_usa_estampas`). Essa identidade permite que vendas novas, vendas editadas e pedidos aprovados gerem movimentacoes de estoque por vinculo, sem depender da descricao textual.

O estoque usa `estoque_movimentacoes` como fonte de verdade. Entradas manuais entram positivas; saidas de venda e transferencias entram negativas. O saldo exibido e sempre a soma das movimentacoes por vinculo e pode ficar negativo.

Quando uma saida de venda excede o saldo disponivel antes da baixa, `reset_sale_stock_movements` cria uma linha em `estoque_ordens` para a quantidade faltante. Edicao ou exclusao da venda remove as ordens automaticas daquela venda e recalcula a partir dos itens atuais.

Tecidos possuem `fornecedor_id` opcional. Essa relacao alimenta o relatorio `Estoque > Resumo fornecedor`, que agrega vendas por tecido e calcula custo vendido com `custo_override` do vinculo ou `tecidos.custo_base`. `Estoque > Mais vendidos` agrega quantidade vendida por vinculo e renderiza barras proporcionais em texto.

## Pedidos

Pedidos ficam persistidos em `pedidos` e `pedido_itens` com status `pendente` ou `aprovado`. Ao gerar um pedido, o app salva os itens, cria um PDF em `Documents\Razai\pedidos` e abre o compartilhamento nativo do Windows com o PDF anexado quando solicitado. Ao aprovar um pedido pago, os itens sao registrados em `vendas` e o pedido passa para `aprovado`.

A escrita do PDF de pedido e tratada como trabalho potencialmente lento/falhavel. O fluxo protege panics da biblioteca de PDF para nao derrubar a TUI e roda em worker em segundo plano por `BackgroundTask`, igual ao padrao de checklist, upload de imagens e Shopee. Enquanto o worker roda, a tela continua responsiva e exibe progresso/status. A pasta fica fora do workspace para nao disparar reinicio quando o app roda por `cargo watch`. O worker gera o PDF a partir dos itens salvos no banco e bloqueia alteracoes/aprovacao do mesmo pedido enquanto estiver em andamento, evitando `pdf_path` desatualizado.

Quando um pedido e aberto pelo historico, a acao `[Cancelar Pedido]` confirma e remove o registro de `pedidos`; os itens saem por `ON DELETE CASCADE`. Arquivos PDF ja gerados nao sao removidos automaticamente.

O compartilhamento de pedido segue o padrao de [PDF e Integracao Nativa do Windows](pdf-windows-integration.md): o worker apenas grava o PDF e o drain abre a Share UI. O sucesso so e reportado quando o Windows dispara `DataRequested`; se a UI nativa nao abrir, o app seleciona o PDF no Explorer e registra detalhes em `%TEMP%\razai_pdf_*.log`.

## Background tasks

Operacoes lentas usam `BackgroundTask<T>` como slot tipado por dominio. Cada fluxo continua tendo seu proprio resultado e drain, mas o controle de `started_at`, `Receiver`, `is_running()` e `try_recv()` fica centralizado. A primeira versao nao implementa fila global, cancelamento ou progresso percentual; cada dominio permite uma tarefa ativa por vez.

## Documentos

A aba `Documentos` concentra documentos operacionais que nao alteram o banco. O fluxo `Imprimir Checklist` permite selecionar tecidos e gerar um PDF em `Documents\Razai\checklists` com os vinculos cadastrados. A pasta fica fora do workspace para nao disparar reinicio quando o app roda por `cargo watch`.

O PDF e montado por `src/app/documentos/pdf.rs` com uma tabela por tecido selecionado. Cada tabela contem thumbnail da cor, tecido, nome da cor e checkbox de conferencia. Antes de desenhar uma tabela, o gerador calcula a altura necessaria e inicia nova pagina quando a tabela nao cabe no espaco restante, evitando cortes de pagina no meio de uma tabela sempre que possivel. Depois que o worker grava o arquivo, o drain aplica o padrao de [PDF e Integracao Nativa do Windows](pdf-windows-integration.md): tenta imprimir e, se nao houver suporte, abre o PDF como fallback.

## Shopee

A integracao Shopee fica centralizada em `src/shopee.rs`.

Responsabilidades:

- carregar credenciais `SHOPEE_*` do `.env`;
- assinar chamadas publicas e shop APIs com HMAC-SHA256;
- manter `access_token` e `refresh_token` atualizados;
- iniciar callback local em `SHOPEE_CALLBACK_ADDR`;
- detectar/iniciar ngrok e persistir URLs publicas;
- separar OAuth (`/shopee/auth` e `/shopee/callback`) do push/webhook (`/shopee/push`);
- consultar anuncios/modelos da Shopee e agrupar estoque por SKU;
- persistir politicas de estoque online por `item_id/model_id`;
- sincronizar o SKU selecionado para `0` ou `100` via `product/update_stock`;
- reconciliar politicas ativas quando o operador pressiona `C` ou quando `/shopee/push` recebe webhook.

Fluxo de estoque:

1. `product/get_item_list` lista itens `NORMAL`.
2. `product/get_item_base_info` busca dados em lotes de ate 50.
3. `product/get_model_list` busca modelos quando o item possui variacoes.
4. O app agrupa primeiro por SKU Pai (`item_sku`) e depois por variacao (`model_sku` ou `item_sku`), normalizado com `trim + uppercase`.
5. O operador expande o SKU Pai e alterna a variacao entre alvo `0` e alvo `100`; esse alvo e salvo em `shopee_stock_policies`.
6. A confirmacao por `Enter` atualiza apenas a variacao selecionada.
7. A reconciliacao por `C` ou por `/shopee/push` consulta novamente a Shopee e reaplica o alvo salvo quando o estoque remoto divergir.

O webhook e tratado como gatilho de reconciliacao, nao como fonte de verdade. O app precisa estar aberto, com callback/ngrok ativo e URL de push cadastrada na Shopee. Grupos multi-location permanecem bloqueados para sync automatico.

Fluxo de anuncio:

- A aba ja documenta a sequencia obrigatoria para produto local, categoria, atributos, imagens, logistica, estoque, GTIN e fiscal BR.
- A publicacao final planejada e `product/add_item` com `item_status=NORMAL`.
- Requisitos detalhados ficam em `docs/ShopeeDocs/`.
