# Estoque Online Shopee por SKU Pai

## Objetivo

Agrupar anuncios/modelos da Shopee primeiro pelo SKU Pai do anuncio (`item_sku`) e, dentro dele, pelas variacoes (`model_sku`). A sincronizacao aplica estoque padrao somente na variacao selecionada:

- toggle desligado: grava `0` em cada item/modelo do SKU;
- toggle ligado: grava `100` em cada item/modelo do SKU.

O estoque somado atual do SKU Pai e da variacao e mostrado apenas como referencia.

## Endpoints usados

- `GET /api/v2/product/get_item_list`
- `GET /api/v2/product/get_item_base_info`
- `GET /api/v2/product/get_model_list`
- `POST /api/v2/product/update_stock`

## Performance de carregamento

- A listagem inicial de IDs continua paginada pela Shopee.
- As consultas de detalhes (`get_item_base_info`) rodam em lotes de ate 50 itens com concorrencia controlada.
- As consultas de variacoes (`get_model_list`) tambem rodam em paralelo com limite interno de concorrencia.
- O limite evita disparar chamadas demais ao mesmo tempo e reduz o tempo total em lojas com muitos anuncios com variacao.

## Regra de agrupamento hierarquico

- Itens com variacao usam `item_sku` como SKU Pai.
- O SKU filho de estoque usa primeiro o tier 1 da Shopee (`tier_variation[0]`), que no fluxo de anuncios Razai e `Cor`.
- Quando a Shopee nao retorna tier 1 suficiente, o sistema usa fallback pelo `model_sku`.
- No fallback, quando `model_sku` vem com tamanho no prefixo, como `050-BORDO`, `1M-BORDO` ou `0,5M-BORDO`, o SKU da variacao de estoque vira apenas a cor (`BORDO`).
- Quando o fallback por `model_sku` nao tem prefixo de tamanho reconhecido, ele continua sendo usado inteiro.
- Itens sem variacao usam `item_sku` como SKU Pai e tambem como SKU da variacao.
- SKU e normalizado com `trim` e uppercase.
- SKUs vazios nao entram no agrupamento.
- A tela inicia com os SKUs Pai recolhidos.
- `Enter` em SKU Pai expande/recolhe.
- `Enter` em variacao confirma sync somente da variacao selecionada.

## Regra de sincronizacao

- Item sem variacao usa `model_id=0`.
- Item/modelo com um unico `location_id` preserva esse valor no update.
- Item/modelo com multiplos `location_id` fica bloqueado para sync automatico.
- Toda sincronizacao exige confirmacao antes de chamar a Shopee.
- A confirmacao no TUI aplica somente a cor/SKU filho selecionado, nunca o SKU Pai inteiro.
- Quando uma cor possui varios tamanhos na Shopee, a sincronizacao atualiza todos os modelos dessa cor, porque o corte por tamanho e tratado no pedido.
