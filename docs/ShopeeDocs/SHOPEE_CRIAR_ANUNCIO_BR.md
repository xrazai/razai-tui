# Criar Anuncio Shopee BR

## Sequencia do fluxo

1. Selecionar produto local do Razai.
2. Informar preco por metro do anuncio.
3. Enviar imagem principal definida em `SHOPEE_DEFAULT_IMAGE_PATH` ou, se ausente, a primeira imagem encontrada em `Pictures`.
4. Criar item base como `NORMAL`.
5. Inicializar variacoes `Cor x Tamanho`.
6. Confirmar item/modelos pela API.

## Endpoints base

- `GET /api/v2/product/get_category`
- `GET /api/v2/product/get_attribute_tree`
- `GET /api/v2/product/get_brand_list`
- `GET /api/v2/product/get_item_limit`
- `GET /api/v2/logistics/get_channel_list`
- `POST /api/v2/media_space/upload_image`
- `POST /api/v2/product/add_item`
- `POST /api/v2/product/init_tier_variation`

## Atualizar anuncio existente por SKU Pai

O fluxo `Shopee > Atualizar anuncios` sincroniza vinculos locais faltantes para anuncios ja publicados:

1. Selecionar tecido local.
2. Buscar anuncios Shopee e filtrar `item_sku == SKU do tecido`.
3. Buscar modelos de cada anuncio com `get_model_list`.
4. Validar estrutura `Cor x Tamanho`.
5. Comparar cores do tier `Cor` com os vinculos locais.
6. Adicionar somente cores faltantes com `update_tier_variation`.
7. Criar os novos modelos com `add_model`, preservando os precos remotos por tamanho.

Endpoints usados:

- `GET /api/v2/product/get_item_list`
- `GET /api/v2/product/get_item_base_info`
- `GET /api/v2/product/get_model_list`
- `POST /api/v2/product/update_tier_variation`
- `POST /api/v2/product/add_model`

Regras:

- Atualiza todos os anuncios encontrados para o mesmo SKU Pai.
- Nao remove cores, tamanhos nem modelos existentes.
- Bloqueia anuncios que nao estejam em `Cor x Tamanho`.
- Bloqueia quando o total final passaria de 100 combinacoes.
- Novas variacoes entram com estoque inicial `1`.
- Novas cores reaproveitam a imagem existente do tier de cor quando disponivel.
- Cada tamanho novo usa o preco ja existente daquele tamanho no proprio anuncio.

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
- `image.image_id_list`
- `tier_variation` e `model` no passo de variacoes

## Padroes Razai

- Categoria: `Roupas Femininas > Tecidos > Outros` (`100416`).
- Marca: `Razai Tecidos`.
- Condicao: `NEW`.
- Publicacao: `NORMAL`.
- SKU principal: SKU do tecido.
- Variacao 1: `Cor`, usando todos os vinculos do tecido.
- Variacao 2: `Tamanho`, iniciando em `0,5m`, `1m`, `2m`, `3m`, `4m`.
- Limite: manter todas as cores e reduzir tamanhos para `cores x tamanhos <= 100`.
- Limite de preco Shopee: como o preco e proporcional por metragem, cortar tamanhos quando a variacao mais cara passaria de 5x a mais barata. Com inicio em `0,5m`, o fluxo publica `0,5m`, `1m` e `2m`.
- SKU da variacao: SKU do vinculo; se ausente, SKU do tecido.
- Estoque por variacao: `1`.
- Preco por variacao: `preco_por_metro * metragem`.
- Peso por variacao: `gramatura_linear_g_m * metragem / 1000`.
- Dimensao base do item: `20 x 20 x 5 cm`.
- Dimensao do frete por variacao: `30 x 30 x 10 cm`.

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
- `same_state_cfop`
- `diff_state_cfop`
- `csosn`
- `origin`
- `measure_unit`

Padrao atual:

- `ncm=55161300`
- `origin=0`
- `same_state_cfop=5102`
- `diff_state_cfop=6102`
- `csosn=102`
- `measure_unit=M`
- `cest=00` somente para satisfazer validacao BR quando a Shopee exigir CEST para o NCM.

Campos opcionais como `ex_tipi`, `fci_num`, `recopi_num`, `additional_info`, PIS/COFINS e `group_item_info` ficam omitidos quando nao forem necessarios.

