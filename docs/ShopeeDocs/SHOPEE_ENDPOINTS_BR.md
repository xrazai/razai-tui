# Endpoints Shopee Open Platform para Vendedores no Brasil

Ultima revisao: 2026-05-06

Este documento lista endpoints da Shopee Open Platform v2 relevantes para vendedores. A disponibilidade real depende do pais, tipo de app, permissoes do parceiro, configuracoes da loja e whitelists da Shopee.

Fontes usadas na pesquisa:

- Shopee Open Platform: https://open.shopee.com/
- Context7 Shopee Open Platform v2: https://context7.com/websites/open_shopee_documents_v2
- Celigo - Available Shopee APIs: https://docs.celigo.com/hc/en-us/articles/18977971017883-Available-Shopee-APIs
- LS Central - Shopee APIs: https://lscentral.azurewebsites.net/Content/LS-Retail/eCommerce/Shopee/Shopee-APIs.htm

## Nota sobre Brasil

Para integracoes locais de vendedores no Brasil, nao trate como base padrao os modulos de `global_product`, `firstmile`, `sbs`, `livestream` e `ads`, salvo quando o app/loja tiver permissao explicita. `global_product` costuma estar ligado a CB seller/cross-border. `firstmile` e `sbs` tambem dependem de fluxos logisticos/warehouse especificos.

Tambem ha relatos recentes de comportamento inconsistente no Brasil ao criar produto com `/api/v2/product/add_item` envolvendo `seller_stock` em contas sem multi-warehouse. Antes de automatizar cadastro de produto, valide o formato de estoque com uma loja real autorizada.

## Autenticacao e Public

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/auth/token/get` | POST | Troca o codigo de autorizacao por `access_token` e `refresh_token`. |
| `/api/v2/auth/access_token/get` | POST | Renova o `access_token` usando `refresh_token`. |
| `/api/v2/public/get_shops_by_partner` | GET | Lista lojas autorizadas para o parceiro. |
| `/api/v2/public/get_merchants_by_partner` | GET | Lista merchants vinculados ao parceiro. |

## Loja e Merchant

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/shop/get_shop_info` | GET | Consulta dados basicos da loja. |
| `/api/v2/shop/get_profile` | GET | Consulta perfil publico/configuracoes da loja. |
| `/api/v2/shop/update_profile` | POST | Atualiza perfil da loja. |
| `/api/v2/shop/get_warehouse_detail` | GET | Consulta detalhes de warehouse da loja, quando aplicavel. |
| `/api/v2/merchant/get_merchant_info` | GET | Consulta dados do merchant. |
| `/api/v2/merchant/get_shop_list_by_merchant` | GET | Lista lojas ligadas ao merchant. |
| `/api/v2/merchant/get_merchant_warehouse_location_list` | GET | Lista localizacoes de estoque/warehouse do merchant. |

## Produtos e Midia

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/product/get_category` | GET | Consulta arvore de categorias disponiveis. |
| `/api/v2/product/get_attributes` | GET | Consulta atributos por categoria. |
| `/api/v2/product/get_attribute_tree` | GET | Consulta arvore completa de atributos. |
| `/api/v2/product/get_brand_list` | GET | Lista marcas disponiveis por categoria. |
| `/api/v2/product/get_item_list` | GET | Lista produtos da loja por status/paginacao. |
| `/api/v2/product/get_item_base_info` | GET | Consulta dados principais do produto. |
| `/api/v2/product/get_item_extra_info` | GET | Consulta dados extras do produto. |
| `/api/v2/product/get_model_list` | GET | Consulta variacoes/modelos do produto. |
| `/api/v2/product/get_item_limit` | GET | Consulta limites de cadastro/publicacao do vendedor. |
| `/api/v2/product/get_item_upload_control` | GET | Consulta restricoes para envio de produtos. |
| `/api/v2/product/get_dts_limit` | GET | Consulta limites de prazo de envio por categoria. |
| `/api/v2/product/support_size_chart` | GET | Verifica suporte a tabela de medidas. |
| `/api/v2/product/get_size_chart_list` | GET | Lista tabelas de medidas. |
| `/api/v2/product/get_size_chart_detail` | GET | Consulta detalhes de uma tabela de medidas. |
| `/api/v2/product/add_item` | POST | Cria produto. Validar estoque no Brasil antes de usar em producao. |
| `/api/v2/product/update_item` | POST | Atualiza dados do produto. |
| `/api/v2/product/delete_item` | POST | Remove produto. |
| `/api/v2/product/init_tier_variation` | POST | Inicializa variacoes. |
| `/api/v2/product/update_tier_variation` | POST | Atualiza estrutura de variacoes. |
| `/api/v2/product/add_model` | POST | Adiciona modelo/variacao. |
| `/api/v2/product/update_model` | POST | Atualiza modelo/variacao. |
| `/api/v2/product/delete_model` | POST | Remove modelo/variacao. |
| `/api/v2/product/update_price` | POST | Atualiza preco. |
| `/api/v2/product/update_stock` | POST | Atualiza estoque. |
| `/api/v2/product/unlist_item` | POST | Pausa ou deslista produto. |
| `/api/v2/product/boost_item` | POST | Impulsiona produto, quando elegivel. |
| `/api/v2/product/get_boosted_item_list` | GET | Lista produtos impulsionados. |
| `/api/v2/product/get_item_promotion_info` | GET | Consulta promocoes aplicadas ao produto. |
| `/api/v2/product/search_item` | GET | Busca produtos da loja. |
| `/api/v2/product/get_comment` | GET | Consulta avaliacoes/comentarios. |
| `/api/v2/product/reply_comment` | POST | Responde comentario. |
| `/api/v2/product/register_brand` | POST | Solicita ou cadastra marca. |
| `/api/v2/product/get_recommend_attributes` | GET | Sugere atributos. |
| `/api/v2/product/get_recommended_weight` | GET | Sugere peso. |
| `/api/v2/product/get_product_info` | GET | Consulta informacoes de produto. |
| `/api/v2/media_space/upload_image` | POST | Faz upload de imagem para produtos/loja. |

## Pedidos

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/order/get_order_list` | GET | Lista pedidos por periodo/status. |
| `/api/v2/order/get_order_detail` | GET | Consulta detalhes completos do pedido. |
| `/api/v2/order/get_shipment_list` | GET | Lista remessas/pacotes. |
| `/api/v2/order/search_package_list` | GET | Busca pacotes. |
| `/api/v2/order/get_package_detail` | GET | Consulta detalhes do pacote. |
| `/api/v2/order/cancel_order` | POST | Cancela pedido. |
| `/api/v2/order/handle_buyer_cancellation` | POST | Aceita ou rejeita cancelamento solicitado pelo comprador. |
| `/api/v2/order/set_note` | POST | Define observacao interna no pedido. |
| `/api/v2/order/split_order` | POST | Divide pedido, quando suportado. |
| `/api/v2/order/unsplit_order` | POST | Desfaz divisao de pedido. |
| `/api/v2/order/get_pending_invoice_order_list` | GET | Lista pedidos pendentes de nota fiscal. Relevante para Brasil. |
| `/api/v2/order/get_buyer_invoice_info` | GET | Consulta dados fiscais do comprador. Relevante para Brasil. |
| `/api/v2/order/add_invoice_data` | POST | Envia dados da nota fiscal. Relevante para Brasil. |
| `/api/v2/order/upload_invoice_doc` | POST | Envia documento da nota fiscal. Relevante para Brasil. |

## Logistica

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/logistics/get_channel_list` | GET | Lista canais logisticos da loja. |
| `/api/v2/logistics/get_shipping_parameter` | GET | Consulta parametros exigidos para envio. |
| `/api/v2/logistics/ship_order` | POST | Solicita envio/expedicao do pedido. |
| `/api/v2/logistics/batch_ship_order` | POST | Solicita expedicao em lote. |
| `/api/v2/logistics/update_shipping_order` | POST | Atualiza dados de envio. |
| `/api/v2/logistics/get_tracking_number` | GET | Obtem codigo de rastreio. |
| `/api/v2/logistics/get_tracking_info` | GET | Consulta rastreamento. |
| `/api/v2/logistics/get_address_list` | GET | Lista enderecos logisticos/coleta. |
| `/api/v2/logistics/set_address_config` | POST | Configura endereco logistico. |
| `/api/v2/logistics/delete_address` | POST | Remove endereco. |
| `/api/v2/logistics/update_channel` | POST | Atualiza canal logistico. |
| `/api/v2/logistics/get_shipping_document_parameter` | GET | Consulta parametros para etiqueta/documento. |
| `/api/v2/logistics/create_shipping_document` | POST | Gera etiqueta/documento. |
| `/api/v2/logistics/get_shipping_document_result` | GET | Consulta status de geracao da etiqueta. |
| `/api/v2/logistics/download_shipping_document` | POST | Baixa etiqueta/documento. |
| `/api/v2/logistics/get_shipping_document_data_info` | GET | Consulta dados do documento logistico. |

## Pagamentos e Financeiro

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/payment/get_escrow_list` | GET | Lista repasses/escrows. |
| `/api/v2/payment/get_escrow_detail` | GET | Consulta detalhes financeiros de um pedido. |
| `/api/v2/payment/get_escrow_detail_batch` | GET | Consulta detalhes financeiros em lote. |
| `/api/v2/payment/get_income_overview` | GET | Consulta visao geral de receitas. |
| `/api/v2/payment/generate_income_report` | POST | Solicita geracao de relatorio financeiro. |
| `/api/v2/payment/get_income_report` | GET | Consulta ou baixa relatorio financeiro. |
| `/api/v2/payment/get_payout_detail` | GET | Consulta detalhes de pagamento/repasses. |
| `/api/v2/payment/get_wallet_transaction_list` | GET | Consulta transacoes da carteira. |
| `/api/v2/payment/get_payment_method_list` | GET | Lista metodos de pagamento. |
| `/api/v2/payment/get_shop_installment_status` | GET | Consulta status de parcelamento da loja. |
| `/api/v2/payment/get_item_installment_status` | GET | Consulta status de parcelamento por item. |
| `/api/v2/payment/set_shop_installment_status` | POST | Altera parcelamento da loja. |
| `/api/v2/payment/set_item_installment_status` | POST | Altera parcelamento por item. |

## Devolucoes e Reembolso

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/returns/get_return_list` | GET | Lista solicitacoes de devolucao/reembolso. |
| `/api/v2/returns/get_return_detail` | GET | Consulta detalhes da solicitacao. |
| `/api/v2/returns/get_available_solutions` | GET | Consulta solucoes disponiveis para o caso. |
| `/api/v2/returns/confirm` | POST | Confirma reembolso/devolucao. |
| `/api/v2/returns/dispute` | POST | Abre disputa. |
| `/api/v2/returns/offer` | POST | Faz contraproposta/solucao. |
| `/api/v2/returns/accept_offer` | POST | Aceita oferta. |
| `/api/v2/returns/query_proof` | GET | Consulta provas/documentos do caso. |

## Categorias da Loja

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/shop_category/get_shop_category_list` | GET | Lista colecoes/categorias da loja. |
| `/api/v2/shop_category/get_item_list` | GET | Lista itens de uma categoria da loja. |
| `/api/v2/shop_category/add_shop_category` | POST | Cria categoria/colecao. |
| `/api/v2/shop_category/update_shop_category` | POST | Atualiza categoria/colecao. |
| `/api/v2/shop_category/delete_shop_category` | POST | Remove categoria/colecao. |
| `/api/v2/shop_category/add_item_list` | POST | Adiciona itens a categoria. |
| `/api/v2/shop_category/delete_item_list` | POST | Remove itens da categoria. |

## Promocoes

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/discount/get_discount_list` | GET | Lista promocoes de desconto. |
| `/api/v2/discount/get_discount` | GET | Consulta detalhes da promocao. |
| `/api/v2/discount/add_discount` | POST | Cria promocao. |
| `/api/v2/discount/update_discount` | POST | Atualiza promocao. |
| `/api/v2/discount/delete_discount` | POST | Remove promocao. |
| `/api/v2/discount/end_discount` | POST | Encerra promocao. |
| `/api/v2/discount/add_discount_item` | POST | Inclui item na promocao. |
| `/api/v2/discount/update_discount_item` | POST | Atualiza item da promocao. |
| `/api/v2/discount/delete_discount_item` | POST | Remove item da promocao. |
| `/api/v2/voucher/get_voucher_list` | GET | Lista vouchers. |
| `/api/v2/voucher/get_voucher_detail` | GET | Consulta detalhes do voucher. |
| `/api/v2/voucher/add_voucher` | POST | Cria voucher. |
| `/api/v2/voucher/update_voucher` | POST | Atualiza voucher. |
| `/api/v2/voucher/delete_voucher` | POST | Remove voucher. |
| `/api/v2/voucher/end_voucher` | POST | Encerra voucher. |
| `/api/v2/add_on_deal/get_add_on_deal_list` | GET | Lista promocoes de compra adicional. |
| `/api/v2/add_on_deal/get_add_on_deal` | GET | Consulta detalhes de compra adicional. |
| `/api/v2/add_on_deal/add_add_on_deal` | POST | Cria compra adicional. |
| `/api/v2/add_on_deal/update_add_on_deal` | POST | Atualiza compra adicional. |
| `/api/v2/add_on_deal/delete_add_on_deal` | POST | Remove compra adicional. |
| `/api/v2/add_on_deal/end_add_on_deal` | POST | Encerra compra adicional. |
| `/api/v2/bundle_deal/get_bundle_deal_list` | GET | Lista kits/pacotes promocionais. |
| `/api/v2/bundle_deal/get_bundle_deal_detail` | GET | Consulta detalhes do kit. |
| `/api/v2/bundle_deal/create_bundle_deal` | POST | Cria kit promocional. |
| `/api/v2/bundle_deal/update_bundle_deal` | POST | Atualiza kit promocional. |
| `/api/v2/bundle_deal/delete_bundle_deal` | POST | Remove kit promocional. |
| `/api/v2/bundle_deal/end_bundle_deal` | POST | Encerra kit promocional. |
| `/api/v2/follow_prize/get_follow_prize_list` | GET | Lista beneficios por seguir loja. |
| `/api/v2/follow_prize/get_follow_prize_detail` | GET | Consulta detalhe do beneficio. |
| `/api/v2/follow_prize/add_follow_prize` | POST | Cria beneficio por seguir loja. |
| `/api/v2/follow_prize/update_follow_prize` | POST | Atualiza beneficio. |
| `/api/v2/follow_prize/delete_follow_prize` | POST | Remove beneficio. |
| `/api/v2/follow_prize/end_follow_prize` | POST | Encerra beneficio. |
| `/api/v2/top_picks/get_top_picks_list` | GET | Lista colecoes em destaque. |
| `/api/v2/top_picks/get_top_picks` | GET | Consulta uma colecao em destaque. |
| `/api/v2/top_picks/add_top_picks` | POST | Cria colecao em destaque. |
| `/api/v2/top_picks/update_top_picks` | POST | Atualiza colecao em destaque. |
| `/api/v2/top_picks/delete_top_picks` | POST | Remove colecao em destaque. |

## Push e Webhook

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/push/get_app_push_config` | GET | Consulta configuracao de webhook/push. |
| `/api/v2/push/set_app_push_config` | POST | Define URL e eventos de webhook/push. |

## Saude da Conta

| Endpoint | Metodo | Resumo |
|---|---:|---|
| `/api/v2/account_health/get_shop_performance` | GET | Consulta metricas de performance da loja. |
| `/api/v2/account_health/get_shop_penalty` | GET | Consulta penalidades/pontos da loja. |

## Modulos Nao Tratados como Base Brasil

| Modulo | Motivo |
|---|---|
| `global_product` | Geralmente ligado a CB seller/cross-border. |
| `firstmile` | Fluxo de primeira milha, normalmente cross-border. |
| `sbs` | Warehouse/solucoes especificas, depende de elegibilidade. |
| `livestream` | Depende de liberacao especifica. |
| `ads` | Depende de produto/permissao de ads. |
