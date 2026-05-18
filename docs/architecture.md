# Arquitetura

O projeto e uma TUI em Rust com `ratatui`, banco local PostgreSQL e chat lateral via OpenRouter.

## Modulos

| Caminho | Responsabilidade |
| --- | --- |
| `src/main.rs` | Inicializa `.env`, banco, terminal e `App`. |
| `src/app.rs` | Estado principal, loop de eventos e roteamento de renderizacao. |
| `src/app/dados.rs` | Eventos e operacoes da aba Dados. |
| `src/app/vendas.rs` | Eventos e operacoes da aba Vendas. |
| `src/app/configuracoes.rs` | Eventos da aba Configuracoes e leitura de impressoras do Windows. |
| `src/screens/chrome.rs` | Header, tabs, footer e chat. |
| `src/screens/dados.rs` | Renderizacao de listas da aba Dados. |
| `src/screens/dados/forms.rs` | Renderizacao dos formularios de Dados. |
| `src/screens/vendas.rs` | Renderizacao do fluxo de Vendas. |
| `src/screens/configuracoes.rs` | Renderizacao da selecao de impressora. |
| `src/models.rs` | Enums, formularios e regras de calculo. |
| `src/models/sku.rs` | Geracao de SKUs. |
| `src/db.rs` | Queries e comandos PostgreSQL. |
| `src/agent.rs` | Skills do agente e chamada OpenRouter. |

## Limite de tamanho

Evitar arquivos com mais de 600 linhas. Se passar disso e houver um corte claro por dominio, dividir em modulo novo. Manter junto apenas quando a logica for parte integral da mesma responsabilidade.

## Estado e renderizacao

`App` guarda o estado da aplicacao e delega:

- eventos para `src/app/*.rs`
- renderizacao para `src/screens/*.rs`
- persistencia para `src/db.rs`
- contexto de IA para `src/agent.rs`

## Banco local

O banco local usa Docker/PostgreSQL. As migrations ficam em `migrations/`.

O app tambem executa garantias de tabela para estruturas recentes, como `configuracoes` e `estampas`, para funcionar em bancos locais ja criados antes dessas migrations.

## Configuracoes

Configuracoes devem persistir no banco, na tabela `configuracoes`, com pares `chave`/`valor`.

Chaves atuais:

- `receipt_printer`: impressora selecionada para recibos de venda.

## Impressao

A aba `Configuracoes` lista impressoras instaladas no Windows com `Get-Printer`. A impressora selecionada e salva no banco. O envio direto de recibo 80mm sera implementado no fechamento do fluxo de venda/recibo.
