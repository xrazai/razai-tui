# Skills do agente

Cada tela ativa uma skill para orientar o chat lateral. O codigo fica em `src/agent.rs`.

## Globais

| Tela | Skill | Objetivo |
| --- | --- | --- |
| Dashboard | `dashboard` | Interpretar indicadores gerais da loja. |
| Pedidos | `pedidos` | Acompanhar pedidos quando a tela for implementada. |
| Estoque | `estoque` | Consultar e movimentar estoque quando a tela for implementada. |
| Configuracoes | `configuracoes.impressora_recibo` | Configurar impressora termica 80mm para recibos com envio direto. |

## Dados

| Tela | Skill | Objetivo |
| --- | --- | --- |
| Dados menu em Tecido | `dados.tecidos` | Orientar o fluxo de tecidos. |
| Lista de tecidos | `dados.tecidos.lista` | Consultar tecidos e iniciar cadastros. |
| Cadastro/edicao de tecido | `dados.tecidos.cadastro` | Validar campos, SKU e calculos de rendimento/gramatura. |
| Dados menu em Cores | `dados.cores` | Orientar o fluxo de cores. |
| Lista de cores | `dados.cores.lista` | Consultar cores e iniciar cadastros. |
| Cadastro/edicao de cor | `dados.cores.cadastro` | Validar hexadecimal, nome, swatch e SKU. |
| Dados menu em Estampas | `dados.estampas` | Orientar o fluxo de estampas. |
| Lista de estampas | `dados.estampas.lista` | Consultar estampas e iniciar cadastros. |
| Cadastro/edicao de estampa | `dados.estampas.cadastro` | Validar nome e SKU automatico. |
| Dados menu em Vinculos | `dados.vinculos` | Orientar vinculos entre tecido e cor/estampa. |
| Menu de vinculos | `dados.vinculos.menu` | Escolher entre criar ou ver vinculos. |
| Criar vinculos: selecionar tecido | `dados.vinculos.criar.tecido` | Selecionar tecido e determinar tipo de vinculo pelo tipo do tecido. |
| Criar vinculos: selecionar itens | `dados.vinculos.criar.itens` | Marcar cores para tecido liso ou estampas para tecido estampado. |
| Ver vinculos: selecionar tecido | `dados.vinculos.ver.tecido` | Selecionar tecido para consulta. |
| Lista de vinculos | `dados.vinculos.lista` | Consultar cores ou estampas vinculadas ao tecido. |

## Vendas

| Tela | Skill | Objetivo |
| --- | --- | --- |
| Menu de vendas | `vendas.menu` | Iniciar nova venda ou acessar historico. |
| Nova venda: selecionar tecido | `vendas.nova.tecido` | Escolher tecido; o app decide cor ou estampa pelo tipo. |
| Nova venda: selecionar vinculo | `vendas.nova.vinculo` | Escolher cor vinculada ou estampa vinculada. |
| Nova venda: lancamento | `vendas.nova.lancamento` | Informar preco unitario, quantidade e conferir resumo. |
| Historico | `vendas.historico` | Consultar vendas anteriores quando implementado. |

## Regras de atualizacao

- Ao criar uma tela nova, adicionar uma skill especifica em `src/agent.rs`.
- Ao alterar o fluxo de uma tela, atualizar a descricao da skill.
- Ao alterar a matriz de skills, atualizar este documento.
- Skills devem descrever a tarefa da tela, nao detalhes de teclado.
