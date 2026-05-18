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

## Configuracao

Use `.env` para variaveis locais:

```env
DATABASE_URL=postgres://razai:razai_dev@localhost:5432/razai_tui
OPENROUTER_API_KEY=...
OPENROUTER_MODEL=anthropic/claude-sonnet-4.5
```

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
- `Pedidos`: reservado para acompanhamento de pedidos.
- `Dados`: cadastros e vinculos.
- `Estoque`: reservado para estoque.
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

## Agente IA

O chat lateral usa OpenRouter quando `OPENROUTER_API_KEY` esta configurada. Cada tela expõe uma skill ativa para orientar o agente. No Dashboard, a skill `dashboard.master` consulta dados locais e prepara cadastros, vinculos, vendas, historico e configuracoes com confirmacao antes de gravar. Responda `sim` para confirmar uma acao pendente ou `nao` para cancelar. A matriz de skills fica em [docs/skills.md](docs/skills.md).

## Arquitetura

A organizacao atual evita arquivos grandes quando for possivel dividir:

- `src/main.rs`: bootstrap de banco, terminal e app.
- `src/app.rs`: estado principal, loop e roteamento.
- `src/app/`: handlers por dominio.
- `src/screens/`: renderizacao de telas.
- `src/models.rs` e `src/models/`: estado de formularios, enums e regras de SKU.
- `src/db.rs` e `src/db/`: acesso ao Postgres.
- `src/agent.rs`: skills e chamada OpenRouter.
