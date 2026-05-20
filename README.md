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

## Regras de UX da TUI

- Acoes visuais entre colchetes, como `[Confirmar]`, `[Voltar]`, `[Gerar PDF]` e `[Desfazer Vinculo]`, devem ficar separadas do conteudo/listagem por pelo menos uma linha vazia quando dividirem a tela com dados de contexto.
- Essa separacao deve ser feita com uma linha/item vazio proprio e mapeamento correto da selecao. Nao coloque `\n` dentro do texto do item selecionavel, porque isso quebra o destaque visual do `ratatui`.
- Menus compostos apenas por acoes podem permanecer compactos, desde que nao estejam misturados com uma listagem de dados.
- Acoes destrutivas persistidas aparecem em vermelho, ficam no fim do grupo de acoes e sempre pedem confirmacao. `Cancelar` abandona apenas edicao temporaria; remocao/desativacao persistida usa rotulo explicito como `[Excluir]`, `[Cancelar Pedido]` ou `[Desfazer Vinculo]`.

## Abas

- `Dashboard`: agente mestre para consultas e acoes com confirmacao.
- `Vendas`: nova venda, historico, edicao e exclusao.
- `Pedidos`: novo pedido, historico, PDF, compartilhamento nativo do Windows e aprovacao para virar venda.
- `Dados`: cadastros e vinculos.
- `Estoque`: saldos por vinculo, movimentacoes, ordens de estoque e relatorios.
- `Shopee`: conexao Shopee, criacao de anuncio, estoque online por SKU e guia BR.
- `Documentos`: emissao de documentos operacionais, como checklist de vinculos em PDF.
- `Configuracoes`: impressora de recibos.

## Dados

`Dados` possui cinco fluxos:

- `Tecido`: cadastro e edicao de tecidos.
- `Cores`: cadastro e edicao de cores com hexadecimal, swatch e SKU automatico.
- `Estampas`: cadastro e edicao de estampas com SKU automatico.
- `Fornecedor`: cadastro e edicao de fornecedores.
- `Vinculos`: vincula tecidos a cores ou estampas.

Regra de vinculos:

- Tecido `Liso` usa cores cadastradas.
- Tecido `Estampado` usa estampas cadastradas.

Na lista de vinculos, `Enter` abre o detalhe do vinculo selecionado. Cada vinculo aceita quatro imagens salvas no banco local:

- `Imagem Original`
- `Imagem Brand`
- `Imagem Modelo`
- `Imagem Alternativa`

Na lista de vinculos, cada item mostra o progresso de imagens no formato `[n/4]`. No detalhe do vinculo, o painel do agente/chat fica oculto para dar mais area ao cadastro de imagens.

Atalhos do detalhe do vinculo:

- `1`: selecionar `Imagem Original`
- `2`: selecionar `Imagem Brand`
- `3`: selecionar `Imagem Modelo`
- `4`: selecionar `Imagem Alternativa`
- `Cima/Baixo`: alternar slot
- `Tab`: proximo vinculo
- `Shift+Tab`: vinculo anterior
- `[Desfazer Vinculo]`: pedir confirmacao para desativar o vinculo para novos lancamentos
- `Enter`: abrir a janela nativa do Windows para imagem ou confirmar a acao selecionada

Depois de salvar uma imagem, o sistema avanca automaticamente para o proximo slot vazio. Quando o vinculo atual fica completo, avanca para o proximo vinculo com imagens pendentes. O detalhe mostra o progresso no titulo (`Imagens n/4`) e o thumbnail do slot selecionado.

`Desfazer Vinculo` apenas desativa o vinculo para novos lancamentos. Historico de vendas/pedidos permanece como estava, e o registro do vinculo com suas imagens continua preservado no banco.

O preview usa `ratatui-image` com deteccao automatica de protocolo do terminal (`Sixel`, `Kitty`, `iTerm2` ou fallback `Halfblocks`). O protocolo ativo aparece no rodape do detalhe. Para forcar um protocolo, defina `RAZAI_IMAGE_PROTOCOL=auto|sixel|kitty|iterm2|halfblocks` antes de iniciar o app.

Durante o upload, a janela nativa de selecao de arquivo e modal; depois que a imagem e escolhida, o salvamento roda em segundo plano e o painel mostra `Salvando imagem...`. Thumbnails ja gerados ficam em cache de memoria por vinculo/slot e sao invalidados quando uma nova imagem e salva naquele slot.

Arquivos em Google Drive/Drives compartilhados podem estar apenas parcialmente sincronizados. Se o app detectar leitura incompleta ou erro `unexpected end of file`, marque o arquivo/pasta como disponivel off-line, aguarde a sincronizacao terminar, ou copie/baixe a imagem para um disco local antes do upload.

## Lista de Precos

`Dados > Lista de Precos` centraliza valores operacionais de tecido:

1. `Custo Base`
2. `Atacado`
3. `Varejo`

O cadastro de tecido permite escolher fornecedor, usado pelo resumo de fornecedor em Estoque. O custo base geral fica em `Lista de Precos > Custo Base`; precos de venda ficam em `Atacado` e `Varejo`.

Cada lista permite:

- definir o valor base do tecido;
- abrir `[Vinculos / Excecoes]`;
- informar um valor especifico para um vinculo quando uma cor/estampa fugir do valor geral;
- apagar o valor especifico para voltar a usar a base;
- digitar o mesmo valor da base para remover o override e manter origem `base`.

Nas listas de primeiro nivel, tecidos com base vazia ainda devem mostrar se existem excecoes, incluindo contagem e faixa de menor/maior valor quando disponivel. Isso evita a impressao de que o tecido esta vazio quando valores individuais ja foram cadastrados.

## Vendas

O fluxo de nova venda e:

1. Selecionar tecido.
2. O sistema carrega automaticamente os vinculos corretos:
   - cores para tecido `Liso`
   - estampas para tecido `Estampado`
3. Selecionar o vinculo.
4. Escolher preco unitario (`Atacado`, `Varejo` ou `Manual`) e informar quantidade.
5. Conferir o resumo do pedido.
6. Finalizar, ou finalizar e imprimir recibo direto na impressora configurada.

O `Resumo do pedido` aparece apenas na tela de lancamento, inclusive quando uma venda do historico esta aberta para edicao. Use `Tab` para focar o resumo, `Cima/Baixo` para selecionar um lancamento, `Enter` para editar e `Delete` para excluir com confirmacao.

No `Historico de Vendas`, o periodo padrao e o dia atual. Ajuste `Data inicio` e `Data fim` no formato `AAAA-MM-DD` e pressione `Enter` para recarregar. Na lista, `Enter` abre a venda selecionada para edicao. A tela de edicao permite salvar alteracoes, salvar e imprimir, cancelar ou excluir com confirmacao.

Toda venda finalizada grava uma saida de estoque por vinculo (`tecido + cor/estampa`). Ao editar ou excluir uma venda, as movimentacoes de estoque e ordens automaticas daquela venda sao recalculadas. O saldo pode ficar negativo.

## Pedidos

Pedidos usam o mesmo fluxo de lancamento de vendas, mas geram uma pendencia em vez de uma venda imediata:

1. Selecionar tecido e vinculo.
2. Escolher preco unitario (`Atacado`, `Varejo` ou `Manual`) e informar quantidade.
3. Gerar pedido.
4. O sistema salva o pedido como `pendente`, gera um PDF em `Documents\Razai\pedidos` em segundo plano e tenta abrir o compartilhamento nativo do Windows com o PDF anexado quando solicitado.
5. Depois do pagamento, abra o pedido no historico e aprove para converter em venda.

Enquanto o PDF e gerado, a TUI continua responsiva e mostra um indicador de progresso. Se a geracao do PDF falhar internamente, a TUI continua aberta e exibe erro no status. Esse fluxo usa o mesmo slot `BackgroundTask` aplicado a upload de imagens, checklist e operacoes Shopee. O arquivo fica fora do workspace para nao reiniciar o app quando ele estiver rodando com `cargo watch`. O PDF e montado a partir dos itens salvos no banco; enquanto ele estiver em andamento, o mesmo pedido nao pode ser alterado ou aprovado.

Ao abrir um pedido pelo historico, `[Cancelar Pedido]` pede confirmacao e remove o pedido da listagem. O PDF ja gerado permanece na pasta de documentos.

O compartilhamento usa a Windows Share UI via WinRT (`DataTransferManager`). O status so informa que o compartilhamento abriu quando o Windows solicita os dados do PDF. Se o painel nativo nao abrir, o app seleciona o PDF no Explorer, mostra o caminho no status e registra detalhes em `%TEMP%\razai_pdf_*.log`.

No resumo do pedido, use `Tab` para focar os lancamentos, `Cima/Baixo` para selecionar, `Enter` para editar preco/quantidade e `Delete` para remover. Pedido pendente nao altera estoque; a baixa acontece quando ele e aprovado e convertido em venda. Se a aprovacao vender acima do saldo disponivel, o app cria uma ordem de estoque para a quantidade faltante.

## Estoque

A aba `Estoque` abre com:

1. `Ver todo o estoque`
2. `Ver ordens de estoque`
3. `Ver resumo fornecedor`
4. `Ver mais vendidos`

`Ver todo o estoque` mostra saldo por vinculo, agrupado por tecido. Ao abrir um vinculo, o operador pode registrar `Entrada` ou `Transferencia`. Entradas aumentam saldo; transferencias reduzem saldo sem calculo financeiro. O saldo e calculado por movimentacoes e aceita valores negativos.

`Ver ordens de estoque` lista pendencias automaticas criadas quando uma venda, ou pedido aprovado como venda, baixa mais do que o saldo disponivel. A ordem nasce `pendente`, nao altera saldo e pode ser direcionada a um fornecedor, concluida ou cancelada.

`Ver resumo fornecedor` permite escolher fornecedor cadastrado e filtrar por periodo no topo (`AAAA-MM-DD`). O relatorio mostra tecidos vendidos daquele fornecedor, quantidade vendida e custo total vendido, usando custo especifico do vinculo quando existir ou custo base do tecido.

`Ver mais vendidos` mostra um ranking geral por vinculo vendido (`tecido + cor/estampa + SKU`) com barras proporcionais em caracteres para comparar quantidades.

## Documentos

A aba `Documentos` fica antes de `Configuracoes` e possui:

1. `Imprimir Checklist`

O checklist permite marcar um ou mais tecidos com `Space` e gerar o PDF com `Ctrl+Enter` ou pela opcao `[Gerar PDF]`. O arquivo e salvo fora do workspace, em `Documents\Razai\checklists`, para nao reiniciar o app quando ele estiver rodando com `cargo watch`.

Depois de gerar, o sistema chama a acao de impressao do Windows para o PDF a partir do loop principal da TUI. Se o visualizador padrao nao oferecer impressao direta, o sistema tenta abrir o PDF como fallback e diferencia esse caso no status.

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
- estoque remoto, disponivel e reservado;
- alvo local persistido (`0` ou `100`);
- status visual (`zerado`, `zerar pendente`, `ativo`, `sync pendente` ou `bloqueado`).

Para acelerar lojas com muitos anuncios, os detalhes dos itens e as variacoes sao buscados em paralelo com concorrencia controlada.

Controles:

- `Enter`: se ainda nao carregou, busca o estoque; em SKU Pai, expande/recolhe; em variacao, confirma sync da variacao.
- `Cima/Baixo`: navega por pais e variacoes visiveis.
- `Space`: alterna a variacao selecionada entre alvo `0` e alvo `100` e salva essa politica no banco local.
- `C`: reconcilia agora todas as politicas Shopee ativas, consultando o estoque remoto e reenviando `update_stock` quando o remoto divergir do alvo local.
- `R`: limpa a lista carregada para recarregar no proximo `Enter`.

A sincronizacao manual por `Enter` altera apenas a variacao selecionada e exige confirmacao. Itens sem variacao usam `model_id=0`. Grupos multi-location ficam bloqueados para atualizacao automatica.

As politicas ficam em `shopee_stock_policies`. O webhook `/shopee/push` e tratado como gatilho de reconciliacao: ao receber push, o app consulta novamente a Shopee e reaplica os alvos salvos quando necessario. Para isso funcionar, o app precisa estar aberto, com callback/ngrok ativo e a URL de push cadastrada na Shopee.

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
