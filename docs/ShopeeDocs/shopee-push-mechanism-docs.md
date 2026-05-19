# Shopee Open Platform - Push Mechanism

Gerado em 2026-05-11 a partir da documentacao oficial da Shopee Open Platform acessada via `@chrome`.

Fonte oficial usada: `https://open.shopee.com/push-mechanism/<id>`.

> Aviso Brasil: a referencia de Push Mechanism e global. Eventos com prefixo `fbs_br_` sao explicitamente ligados a FBS Brasil e fluxos fiscais/logisticos brasileiros. Os demais eventos podem depender de categoria do app, permissoes, programa habilitado, recurso logistico e disponibilidade da loja BR. Antes de producao, valide inscricao do push no Console da Shopee, callback HTTPS, permissao do app e loja brasileira autorizada.

## Resumo

- Total coletado: 34 mecanismos push.
- A maioria usa timeout de 3s, permite mensagem repetida e nao garante ordem.
- `webchat_push` e excecao importante: timeout 2s, ordem garantida `Yes`, retry `1s,2s,3s`.
- Trate todos os push como notificacao de mudanca, nao como fonte unica da verdade.
- Para estados criticos, confirme via API REST correspondente depois de receber o push.

## Como implementar o receptor

1. Configure uma URL HTTPS de callback no Console da Shopee.
2. Responda rapidamente, idealmente validando e enfileirando o payload.
3. Deduplicate por `code`, `shop_id`, `timestamp` e identificadores dentro de `data`.
4. Nao dependa da ordem de chegada quando `Sequence Guaranteed = No`.
5. Consulte a API REST para confirmar o estado final de pedido, produto, estoque, autorizacao, penalidade ou FBS.
6. Salve o payload bruto para auditoria e reprocessamento.

## Campos comuns

| Campo | Tipo | Uso |
|---|---|---|
| `shop_id` | int | Identificador da loja. Pode aparecer no corpo raiz e dentro de `data`. |
| `code` | int | Codigo numerico do mecanismo push. |
| `timestamp` | timestamp | Momento em que a Shopee enviou a mensagem. |
| `data` | object | Conteudo especifico do evento. |

## Indice dos mecanismos

| Categoria | Codigo | Mecanismo | O que faz | Brasil |
|---|---:|---|---|---|
| Product Push | 8 | `reserved_stock_change_push` | Mudanca de estoque reservado em promocao/pedido/cancelamento. | Validar disponibilidade BR |
| Product Push | 11 | `video_upload_push` | Resultado de upload de video usado em item/criacao/atualizacao. | Validar disponibilidade BR |
| Product Push | 13 | `brand_register_result` | Resultado de registro/revisao/merge de marca. | Validar disponibilidade BR |
| Product Push | 16 | `violation_item_push` | Item banido, removido pela Shopee ou marcado como deboost, com motivo e sugestao. | Validar disponibilidade BR |
| Product Push | 22 | `item_price_update_push` | Alteracao de preco original/local de item ou variacao. | Validar disponibilidade BR |
| Product Push | 27 | `item_scheduled_publish_failed_push` | Falha ao publicar produto no horario agendado. | Validar disponibilidade BR |
| Order Push | 3 | `order_status_push` | Alteracoes de status de pedido. | Validar disponibilidade BR |
| Order Push | 4 | `order_trackingno_push` | Atualizacao de tracking number de pedido. | Validar disponibilidade BR |
| Order Push | 15 | `shipping_document_status_push` | Atualizacao de status de documento de envio. | Validar disponibilidade BR |
| Order Push | 23 | `booking_status_push` | Alteracoes de status de booking. | Validar disponibilidade BR |
| Order Push | 24 | `booking_trackingno_push` | Atualizacao de tracking number de booking. | Validar disponibilidade BR |
| Order Push | 25 | `booking_shipping_document_status_push` | Status de documento de envio de booking. | Validar disponibilidade BR |
| Order Push | 30 | `package_fulfillment_status_push` | Alteracoes de status de fulfillment de pacote. | Validar disponibilidade BR |
| Order Push | 37 | `courier_delivery_binding_status_push` | Status de tracking de primeira milha para courier delivery. | Validar disponibilidade BR |
| Order Push | 47 | `package_info_push` | Mudancas em pacote, como ship by date, canal logistico e return code. | `return_code` indicado como restrito a ID/SPX Instant & Sameday |
| Return Push | 29 | `return_updates_push` | Mudancas em return/refund: status, solucao, prova do seller e status logistico. | Validar disponibilidade BR |
| Marketing Push | 7 | `item_promotion_push` | Atualizacoes de promocao em item, inclusive lock/unlock de estoque promocional. | Validar disponibilidade BR |
| Marketing Push | 9 | `promotion_update_push` | Atualizacoes gerais de promocao. | Validar disponibilidade BR |
| Shopee Push | 5 | `shopee_updates` | Notificacoes do My Inbox/Seller Center. | Validar disponibilidade BR |
| Shopee Push | 12 | `open_api_authorization_expiry` | Autorizacoes de shops, merchants e users que expiram em ate uma semana. | Validar disponibilidade BR |
| Shopee Push | 1 | `shop_authorization_push` | Loja/merchant autorizou o app. | Validar disponibilidade BR |
| Shopee Push | 2 | `shop_authorization_canceled_push` | Loja/merchant/user desautorizou o app. | Validar disponibilidade BR |
| Shopee Push | 28 | `shop_penalty_update_push` | Atualizacoes de penalty point ou punishment tier. | Validar disponibilidade BR |
| Shopee Push | 38 | `video_upload_result_push` | Status final de upload de video: `SUCCEEDED`, `FAILED` ou `CANCELLED`. | Validar disponibilidade BR |
| Webchat Push | 10 | `webchat_push` | Mensagens e notificacoes de chat. | Validar disponibilidade BR |
| Consignment Service Push | 21 | `inbound_status_push` | Mudanca de status de inbound. | Validar se o programa existe no BR |
| Consignment Service Push | 18 | `supplier_create_product_push` | Produto de fornecedor criado com sucesso. | Validar se o programa existe no BR |
| Consignment Service Push | 19 | `supplier_prouduct_review_result_push` | Resultado de revisao de produto de fornecedor. | Validar se o programa existe no BR |
| Consignment Service Push | 20 | `purchase_order_Push` | Novo purchase order. | Validar se o programa existe no BR |
| Fulfillment by Shopee Push | 36 | `fbs_sellable_stock` | Mudanca em estoque vendavel no armazem FBS. | Validar FBS BR |
| Fulfillment by Shopee Push | 33 | `fbs_br_invoice_error_push` | Falha de emissao de invoice/NF em fluxos FBS. | Explicitamente BR/FBS |
| Fulfillment by Shopee Push | 34 | `fbs_br_block_shop_push` | Loja bloqueada por erro de invoice/NF no FBS. | Explicitamente BR/FBS |
| Fulfillment by Shopee Push | 35 | `fbs_br_block_sku_push` | SKU/produto bloqueado por erro de invoice/NF no FBS. | Explicitamente BR/FBS |
| Fulfillment by Shopee Push | 31 | `fbs_br_invoice_issued_push` | Invoice/NF emitida em fluxo FBS BR. | Explicitamente BR/FBS |

## Detalhes por categoria

### Product Push

`reserved_stock_change_push` (`code=8`, `/push-mechanism/5`)

Notifica mudanca de estoque reservado. A documentacao mostra acoes como `place_order` e `cancel_order`, com `item_id`, `variation_id`, `changed_values`, `promotion_type`, `promotion_id`, `ordersn` e `update_time`. Use para reagir a reserva/liberacao de estoque, mas confirme estoque final pelas APIs de produto/estoque quando for atualizar ERP.

`video_upload_push` (`code=11`, `/push-mechanism/11`)

Retorna resultado de upload de video usado em criacao/atualizacao de item. O payload inclui identificador da sessao/upload, resultado (`SUCCEEDED`/`FAILED`) e detalhes de video ou erro. Para fluxos novos, compare tambem com `video_upload_result_push`, que cobre status final do upload de video.

`brand_register_result` (`code=13`, `/push-mechanism/13`)

Informa resultado de registro de marca. A pagina mostra casos de marca aprovada, rejeitada ou combinada com marca existente. Quando rejeitada ou combinada, produtos historicos podem ter marca alterada para `No brand` ou para a marca combinada.

`violation_item_push` (`code=16`, `/push-mechanism/18`)

Notifica violacoes quando item fica `BANNED`, `SHOPEE_DELETE` ou deboost. Inclui tipo de violacao, motivo, sugestao, prazo de correcao e detalhes de categoria sugerida quando aplicavel. Deve acionar revisao operacional imediata.

`item_price_update_push` (`code=22`, `/push-mechanism/25`)

Dispara quando o seller atualiza `original_price` do item/modelo. Update log de 2025-09-08 indica suporte a `local_price`. Use para sincronizar preco em ERP e auditar alteracoes externas.

`item_scheduled_publish_failed_push` (`code=27`, `/push-mechanism/30`)

Notifica falha na publicacao agendada de produto. Deve gerar alerta para revisar o item, categoria, atributos obrigatorios e regras de publicacao.

### Order Push

`order_status_push` (`code=3`, `/push-mechanism/1`)

Notifica mudancas de status de pedido, inclusive cancelamentos antes do envio. A documentacao indica campo `completed_scenario` para diferenciar cenarios de pedido completado. Ao receber, consulte Order API para estado completo do pedido.

`order_trackingno_push` (`code=4`, `/push-mechanism/2`)

Notifica atualizacao de tracking number, evitando polling continuo em `v2.logistics.get_tracking_number`. Use para disparar impressao/documento/logistica, mas confirme dados logisticos antes de acao irreversivel.

`shipping_document_status_push` (`code=15`, `/push-mechanism/17`)

Notifica status de shipping document, por exemplo `READY`. Use para baixar/imprimir documento de envio quando estiver pronto.

`booking_status_push` (`code=23`, `/push-mechanism/26`)

Equivalente a status de pedido para fluxos de booking. Inclui cancelamentos antes do envio.

`booking_trackingno_push` (`code=24`, `/push-mechanism/27`)

Notifica tracking number em booking. Relacionado a `v2.logistics.get_booking_tracking_number`.

`booking_shipping_document_status_push` (`code=25`, `/push-mechanism/28`)

Notifica status de documento de envio de booking.

`package_fulfillment_status_push` (`code=30`, `/push-mechanism/33`)

Notifica status de fulfillment de pacote, com `ordersn`, `package_number`, `fulfillment_status` e `update_time`. Use para acompanhar pacote dentro do pedido.

`courier_delivery_binding_status_push` (`code=37`, `/push-mechanism/34`)

Notifica status de tracking number de primeira milha para courier delivery. A documentacao cita status como `ORDER_RECEIVED`.

`package_info_push` (`code=47`, `/push-mechanism/44`)

Notifica mudancas de informacoes do pacote: `ship_by_date`, `logistics_channel_id` ou `return_code`. Aviso regional importante: a documentacao diz que `return_code` se aplica a pacotes sob SPX Instant & Sameday na regiao ID. Para Brasil, nao assumir suporte a `return_code` sem validacao.

### Return Push

`return_updates_push` (`code=29`, `/push-mechanism/32`)

Notifica alteracoes de Return/Refund em `return_status`, `return_solution`, `seller_proof_status` e `logistics_status`. Deve acionar sincronizacao com modulo de atendimento e pos-venda.

### Marketing Push

`item_promotion_push` (`code=7`, `/push-mechanism/6`)

Notifica alteracoes de promocao em item e estoque reservado promocional. A documentacao mostra acoes `promo_lock_stock`, `promo_cancelled` e `promo_end`. Afeta estoque normal e promocional.

`promotion_update_push` (`code=9`, `/push-mechanism/7`)

Notifica atualizacao de promocao, com acoes como `added_in_promo`, `removed_from_promo` e `promo_time_updated`. Use para sincronizar calendario de campanhas e exposicao comercial.

### Shopee Push

`shopee_updates` (`code=5`, `/push-mechanism/3`)

Notifica mensagens do `My Inbox` no Seller Center. Inclui titulo e conteudo, mas nao detalhes completos.

`open_api_authorization_expiry` (`code=12`, `/push-mechanism/12`)

Lista shops, merchants e users cuja autorizacao expira em ate uma semana. Deve gerar renovacao proativa de autorizacao.

`shop_authorization_push` (`code=1`, `/push-mechanism/15`)

Notifica autorizacao de shop ou merchant ao app. Deve iniciar criacao/atualizacao de credenciais, tokens e cadastro local.

`shop_authorization_canceled_push` (`code=2`, `/push-mechanism/16`)

Notifica desautorizacao de shop, merchant ou user. Deve desativar sincronizacoes e evitar chamadas com token invalido.

`shop_penalty_update_push` (`code=28`, `/push-mechanism/31`)

Notifica penalty point emitido/removido ou mudanca de punishment tier. Deve alimentar monitoramento de saude da loja.

`video_upload_result_push` (`code=38`, `/push-mechanism/43`)

Notifica quando upload de video chega a status final: `SUCCEEDED`, `FAILED` ou `CANCELLED`. Status intermediarios nao sao enviados; para progresso, a documentacao orienta chamar `v2.media.get_video_upload_result`.

### Webchat Push

`webchat_push` (`code=10`, `/push-mechanism/10`)

Notifica mensagens de chat e notificacoes. Diferente da maioria dos pushes: timeout 2s, ordem garantida `Yes` e retry `1s,2s,3s`. Deve ser processado com baixa latencia e fila dedicada se o atendimento depender dele.

### Consignment Service Push

`inbound_status_push` (`code=21`, `/push-mechanism/20`)

Notifica mudanca de status de inbound. Programa/servico pode nao estar disponivel para toda loja BR.

`supplier_create_product_push` (`code=18`, `/push-mechanism/22`)

Notifica sucesso na criacao de produto de fornecedor.

`supplier_prouduct_review_result_push` (`code=19`, `/push-mechanism/23`)

Notifica alteracao no resultado de revisao de produto de fornecedor. O nome oficial esta escrito como `prouduct` na documentacao.

`purchase_order_Push` (`code=20`, `/push-mechanism/24`)

Notifica novo purchase order. O nome oficial usa `Push` com `P` maiusculo.

### Fulfillment by Shopee Push

`fbs_sellable_stock` (`code=36`, `/push-mechanism/36`)

Notifica mudanca no estoque vendavel do armazem FBS, excluindo estoque em zona isolada/danificada. Para Brasil, validar se a loja usa FBS e se o armazem/regiao retornados se aplicam.

`fbs_br_invoice_error_push` (`code=33`, `/push-mechanism/38`)

Evento explicitamente BR/FBS. Notifica falha de emissao de invoice/NF em Inbound Request, RTS Request, sales orders e move transfer orders. Deve acionar rotina fiscal com prioridade.

`fbs_br_block_shop_push` (`code=34`, `/push-mechanism/39`)

Evento explicitamente BR/FBS. Notifica bloqueio de loja por erro de invoice/NF. A documentacao indica que nao e permitido criar novo Inbound Request e o estoque do armazem nao fica vendavel.

`fbs_br_block_sku_push` (`code=35`, `/push-mechanism/40`)

Evento explicitamente BR/FBS. Notifica bloqueio de produto/SKU por erro de invoice/NF. Tambem impacta Inbound Request e estoque vendavel.

`fbs_br_invoice_issued_push` (`code=31`, `/push-mechanism/41`)

Evento explicitamente BR/FBS. Notifica emissao de invoice/NF em fluxo FBS BR. Use para conciliar pedido/remessa/retorno simbolico conforme o tipo de documento.

## Prioridades para gestao basica

Para uma operacao basica de seller/ERP, priorize:

1. Autorizacao: `shop_authorization_push`, `shop_authorization_canceled_push`, `open_api_authorization_expiry`.
2. Pedido/logistica: `order_status_push`, `order_trackingno_push`, `shipping_document_status_push`, `package_fulfillment_status_push`, `package_info_push`.
3. Produto/estoque: `reserved_stock_change_push`, `item_price_update_push`, `violation_item_push`.
4. Pos-venda: `return_updates_push`.
5. Brasil/FBS, se aplicavel: todos os `fbs_br_*` e `fbs_sellable_stock`.

