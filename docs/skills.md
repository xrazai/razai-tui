# Capacidades do Razai Master

O chat lateral usa um unico agente: **Razai Master**. A tela atual define apenas o contexto preferencial exibido no painel e enviado ao OpenRouter; o agente continua tendo acesso ao contexto global do projeto e as mesmas rotinas guiadas.

## Capacidades

| Capacidade | Objetivo |
| --- | --- |
| `dashboard.master` | Consultar dados locais e preparar acoes de qualquer area com confirmacao. |
| `dados.tecidos` | Criar, consultar e orientar cadastro de tecidos, incluindo SKU e calculos de rendimento/gramatura. |
| `dados.cores` | Criar e consultar cores, validando hexadecimal e SKU. |
| `dados.estampas` | Criar e consultar estampas com SKU automatico. |
| `dados.vinculos` | Criar e consultar vinculos entre tecido e cor/estampa. |
| `dados.vinculos.imagens` | Orientar o upload local das quatro imagens de um vinculo e a leitura do thumbnail no terminal. |
| `vendas` | Lançar itens, consultar historico, filtrar periodo e abrir venda por id. |
| `pedidos` | Lançar itens, gerar PDF, compartilhar pelo Windows e aprovar pedido como venda. |
| `documentos` | Orientar a emissao de documentos operacionais, como checklist PDF de vinculos por tecido. |
| `configuracoes` | Selecionar impressora de recibos. |
| `estoque` | Consultar e movimentar estoque quando a tela for implementada. |
| `shopee` | Apoiar conexao Shopee, estoque online por SKU, criacao de anuncio e requisitos BR. |

## Fluxos Guiados

O agente pergunta uma informacao por vez quando faltam dados obrigatorios. Toda acao de escrita, vinculo, edicao, exclusao, impressao ou configuracao precisa de confirmacao textual antes de executar.

Acoes locais ja mapeadas:

- cadastrar tecido, cor e estampa;
- criar vinculo tecido + cor/estampa;
- orientar cadastro de imagens no detalhe do vinculo;
- lançar item de venda;
- lançar item de pedido;
- orientar emissao do checklist de vinculos em PDF;
- abrir venda por id;
- filtrar historico por periodo;
- selecionar impressora.

Na aba Shopee, o agente deve orientar sem afirmar que executou alteracoes diretas fora dos fluxos confirmados do app. Para estoque, o fluxo visual do sistema carrega SKUs, alterna `0/100` e confirma sincronizacao do SKU selecionado. Para criacao de anuncio, a orientacao deve seguir os documentos em `docs/ShopeeDocs/`, especialmente campos obrigatorios, imagens e fiscal BR.

## Regras de Atualizacao

- Ao criar uma tela nova, adicionar ou ajustar a capacidade correspondente em `src/agent.rs`.
- Ao alterar um fluxo guiado, atualizar `src/app/agent_actions.rs`.
- Ao alterar a matriz de capacidades, atualizar este documento.
