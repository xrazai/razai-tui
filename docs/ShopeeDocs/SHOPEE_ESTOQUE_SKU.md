# Estoque Online Shopee por SKU

## Objetivo

Agrupar todos os anuncios/modelos da Shopee pelo SKU do vendedor e permitir aplicar estoque padrao por grupo:

- toggle desligado: grava `0` em cada item/modelo do SKU;
- toggle ligado: grava `100` em cada item/modelo do SKU.

O estoque somado atual e mostrado apenas como referencia.

## Endpoints usados

- `GET /api/v2/product/get_item_list`
- `GET /api/v2/product/get_item_base_info`
- `GET /api/v2/product/get_model_list`
- `POST /api/v2/product/update_stock`

## Regra de agrupamento

- Itens com variacao usam `model_sku`.
- Itens sem variacao usam `item_sku`.
- SKU e normalizado com `trim` e uppercase.
- SKUs vazios nao entram no agrupamento.
- SKUs unicos tambem aparecem na tela.

## Regra de sincronizacao

- Item sem variacao usa `model_id=0`.
- Item/modelo com um unico `location_id` preserva esse valor no update.
- Item/modelo com multiplos `location_id` fica bloqueado para sync automatico.
- Toda sincronizacao exige confirmacao antes de chamar a Shopee.
- A confirmacao no TUI aplica somente o grupo de SKU selecionado.
