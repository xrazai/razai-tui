# Criar Anuncio Shopee BR

## Sequencia do fluxo

1. Selecionar produto local do Razai.
2. Escolher categoria Shopee.
3. Preencher dados principais.
4. Preencher atributos obrigatorios da categoria.
5. Enviar imagens obrigatorias.
6. Preencher logistica, estoque e GTIN.
7. Preencher dados fiscais BR.
8. Revisar payload.
9. Publicar como `NORMAL`.

## Endpoints base

- `GET /api/v2/product/get_category`
- `GET /api/v2/product/get_attribute_tree`
- `GET /api/v2/product/get_brand_list`
- `GET /api/v2/product/get_item_limit`
- `GET /api/v2/logistics/get_channel_list`
- `POST /api/v2/media_space/upload_image`
- `POST /api/v2/product/add_item`

## Campos obrigatorios base

- `item_name`
- `description`
- `original_price`
- `weight`
- `dimension.package_height`
- `dimension.package_length`
- `dimension.package_width`
- `category_id`
- `condition`
- `logistic_info`
- `seller_stock`
- `gtin_code`
- `image.image_id_list`
- `attribute_list` com todos os atributos obrigatorios da categoria

## Imagens

- Usar `media_space/upload_image`.
- Formatos aceitos: JPG, JPEG e PNG.
- Tamanho maximo por arquivo: 10 MB.
- Limite operacional: ate 8 imagens por anuncio.
- Usar `scene=normal`.
- Usar `ratio=1:1` por padrao.

## Fiscal BR

Campos fiscais devem ser conferidos pela operacao/contabilidade antes de publicar:

- `ncm`
- `cest`
- `same_state_cfop`
- `diff_state_cfop`
- `csosn`
- `origin`
- `measure_unit`
- `pis`
- `cofins`
- `icms_cst`
- `pis_cofins_cst`
- `federal_state_taxes`
- `operation_type`
- `export_cfop`

Campos opcionais como `ex_tipi`, `fci_num`, `recopi_num`, `additional_info` e `group_item_info` devem ser preenchidos quando aplicaveis.

