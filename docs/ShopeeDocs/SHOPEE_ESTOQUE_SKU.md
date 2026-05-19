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

## Regra de agrupamento hierarquico

- Itens com variacao usam `item_sku` como SKU Pai e `model_sku` como SKU da variacao.
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
- A confirmacao no TUI aplica somente a variacao selecionada, nunca o SKU Pai inteiro.
