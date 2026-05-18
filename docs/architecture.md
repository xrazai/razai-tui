# Arquitetura

O projeto e uma TUI em Rust com `ratatui`, banco local PostgreSQL e chat lateral via OpenRouter.

## Modulos

| Caminho | Responsabilidade |
| --- | --- |
| `src/main.rs` | Inicializa `.env`, banco, terminal e `App`. |
| `src/app.rs` | Estado principal, loop de eventos e roteamento de renderizacao. |
| `src/app/agent_actions.rs` | Acoes pendentes do agente mestre do Dashboard e execucao apos confirmacao. |
| `src/app/dados.rs` | Eventos e operacoes da aba Dados. |
| `src/app/vendas.rs` | Eventos e operacoes da aba Vendas. |
| `src/app/vendas/history.rs` | Filtro de periodo e abertura de vendas do historico. |
| `src/app/vendas/receipt.rs` | Montagem e envio RAW/ESC-POS de recibos. |
| `src/app/configuracoes.rs` | Eventos da aba Configuracoes e leitura de impressoras do Windows. |
| `src/screens/chrome.rs` | Header, tabs, footer e chat. |
| `src/screens/dados.rs` | Renderizacao de listas da aba Dados. |
| `src/screens/dados/forms.rs` | Renderizacao dos formularios de Dados. |
| `src/screens/vendas.rs` | Renderizacao do fluxo de Vendas. |
| `src/screens/configuracoes.rs` | Renderizacao da selecao de impressora. |
| `src/models.rs` | Enums, formularios e regras de calculo. |
| `src/models/sku.rs` | Geracao de SKUs. |
| `src/db.rs` | Queries e comandos PostgreSQL gerais. |
| `src/db/sales.rs` | Persistencia de vendas, itens e historico. |
| `src/agent.rs` | Skills do agente e chamada OpenRouter. |

## Limite de tamanho

Evitar arquivos com mais de 600 linhas. Se passar disso e houver um corte claro por dominio, dividir em modulo novo. Manter junto apenas quando a logica for parte integral da mesma responsabilidade.

## Estado e renderizacao

`App` guarda o estado da aplicacao e delega:

- eventos para `src/app/*.rs`
- renderizacao para `src/screens/*.rs`
- persistencia para `src/db.rs`
- contexto de IA para `src/agent.rs`

## Agente IA

O Dashboard usa a skill `dashboard.master`, que combina contexto de dados, vendas, vinculos e configuracoes. Consultas locais podem ser respondidas diretamente. Acoes que gravam ou alteram dados ficam em `pending_agent_action` e so executam depois de confirmacao textual do usuario (`sim`/`nao`).

Acoes mapeadas inicialmente:

- cadastro de tecido, cor e estampa;
- criacao de vinculo entre tecido e cor/estampa;
- abertura de venda por id;
- filtro de historico por periodo;
- selecao de impressora.

## Banco local

O banco local usa Docker/PostgreSQL. As migrations ficam em `migrations/`.

O app tambem executa garantias de tabela para estruturas recentes, como `configuracoes` e `estampas`, para funcionar em bancos locais ja criados antes dessas migrations.

## Configuracoes

Configuracoes devem persistir no banco, na tabela `configuracoes`, com pares `chave`/`valor`.

Chaves atuais:

- `receipt_printer`: impressora selecionada para recibos de venda.

## Vendas e impressao

A aba `Configuracoes` lista impressoras instaladas no Windows com `Get-Printer`. A impressora selecionada e salva no banco.

Vendas finalizadas sao persistidas em `vendas` e `venda_itens`. O historico inicia filtrado pelo dia atual e permite ajustar `Data inicio` e `Data fim`. O `Resumo do pedido` so aparece no lancamento ou na edicao de uma venda aberta pelo historico; nele, lancamentos podem ser selecionados, editados individualmente ou excluidos com confirmacao. O envio de recibo 80mm usa impressao RAW/ESC-POS direto para a impressora configurada, sem abrir janela de impressao.
