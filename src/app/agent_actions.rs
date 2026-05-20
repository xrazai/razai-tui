use super::App;
use crate::{
    db,
    models::{
        AgentAction, AgentDraft, CorForm, DadosOption, DadosScreen, EstampaForm, Section,
        SelectValue, TIPO_OPTIONS, TecidoForm, VendaItem, VendasScreen, parse_hex_color,
        parse_number,
    },
};

impl App {
    pub(super) fn handle_pending_agent_draft(&mut self, message: &str) -> bool {
        let Some(mut draft) = self.pending_agent_draft.take() else {
            return false;
        };
        merge_draft_from_message(&mut draft, message);
        match draft_next_step(&draft) {
            DraftStep::Ask(question) => {
                self.pending_agent_draft = Some(draft);
                self.chat
                    .messages
                    .push(crate::models::ChatMessage::assistant(question));
            }
            DraftStep::Confirm(action) => {
                let description = describe_action(&action);
                self.pending_agent_action = Some(action);
                self.chat
                    .messages
                    .push(crate::models::ChatMessage::assistant(format!(
                        "{description}\n\nResponda 'sim' para confirmar ou 'nao' para cancelar."
                    )));
            }
        }
        true
    }

    pub(super) fn handle_pending_agent_confirmation(&mut self, message: &str) -> bool {
        let Some(action) = self.pending_agent_action.clone() else {
            return false;
        };
        let normalized = normalize(message);
        if matches!(normalized.as_str(), "sim" | "s" | "confirmar" | "confirma") {
            self.pending_agent_action = None;
            let reply = self.execute_agent_action(action);
            self.chat
                .messages
                .push(crate::models::ChatMessage::assistant(reply));
            return true;
        }
        if matches!(
            normalized.as_str(),
            "nao" | "não" | "n" | "cancelar" | "cancela"
        ) {
            self.pending_agent_action = None;
            self.pending_agent_draft = None;
            self.chat
                .messages
                .push(crate::models::ChatMessage::assistant(String::from(
                    "Acao cancelada.",
                )));
            return true;
        }
        false
    }

    pub(super) fn submit_context_agent_message(&mut self, message: &str) -> bool {
        let Some(draft) = draft_from_context(self, message) else {
            return false;
        };
        self.pending_agent_draft = Some(draft);
        self.handle_pending_agent_draft("")
    }

    pub(super) fn submit_dashboard_agent_message(&mut self, message: &str) {
        if let Some(action) = parse_agent_action(message) {
            let description = describe_action(&action);
            self.pending_agent_action = Some(action);
            self.chat
                .messages
                .push(crate::models::ChatMessage::assistant(format!(
                    "{description}\n\nResponda 'sim' para confirmar ou 'nao' para cancelar."
                )));
            return;
        }
        if self.submit_context_agent_message(message) {
            return;
        }
        if let Some(reply) = self.answer_dashboard_query(message) {
            self.chat
                .messages
                .push(crate::models::ChatMessage::assistant(reply));
            return;
        }

        let context_info = self.active_context();
        let context = self.dashboard_agent_context();
        let fallback = format!(
            "Razai Master. Posso consultar dados locais e preparar acoes com confirmacao. Contexto atual: {}.",
            context_info.capability
        );
        self.spawn_agent_reply(context_info, message.to_string(), context, fallback);
    }

    fn answer_dashboard_query(&self, message: &str) -> Option<String> {
        let normalized = normalize(message);
        if is_creation_intent(&normalized) {
            return None;
        }
        if normalized.contains("quanto") && normalized.contains("vendi") {
            let total = self
                .vendas_historico
                .iter()
                .map(|venda| venda.total)
                .sum::<f64>();
            return Some(format!(
                "No periodo carregado ha {} venda(s), totalizando R${}.",
                self.vendas_historico.len(),
                format_money(total)
            ));
        }
        if normalized.contains("tecido") && !normalized.contains("cad") {
            return Some(format_named_list(
                "Tecidos",
                self.tecidos
                    .iter()
                    .map(|tecido| format!("{} ({})", tecido.nome, tecido.sku)),
            ));
        }
        if normalized.contains("cor") && !normalized.contains("cad") {
            return Some(format_named_list(
                "Cores",
                self.cores.iter().map(|cor| {
                    format!(
                        "{} ({}) {}",
                        cor.nome,
                        cor.sku.as_deref().unwrap_or("sem SKU"),
                        cor.codigo_hex.as_deref().unwrap_or("")
                    )
                }),
            ));
        }
        if normalized.contains("estampa") && !normalized.contains("cad") {
            return Some(format_named_list(
                "Estampas",
                self.estampas.iter().map(|estampa| {
                    format!(
                        "{} ({})",
                        estampa.nome,
                        estampa.sku.as_deref().unwrap_or("sem SKU")
                    )
                }),
            ));
        }
        if normalized.contains("impressora") {
            return Some(match &self.selected_printer {
                Some(printer) => format!("Impressora configurada: {printer}."),
                None => String::from("Nenhuma impressora configurada."),
            });
        }
        None
    }

    fn execute_agent_action(&mut self, action: AgentAction) -> String {
        match action {
            AgentAction::CreateCor { nome, hex } => self.agent_create_cor(&nome, &hex),
            AgentAction::CreateEstampa { nome } => self.agent_create_estampa(&nome),
            AgentAction::CreateTecido {
                nome,
                composicao,
                largura,
                tipo,
                rendimento,
                gramatura_linear,
                gramatura_m2,
            } => self.agent_create_tecido(
                &nome,
                &composicao,
                &largura,
                tipo.as_deref(),
                rendimento.as_deref(),
                gramatura_linear.as_deref(),
                gramatura_m2.as_deref(),
            ),
            AgentAction::CreateVinculo { tecido, item } => {
                self.agent_create_vinculo(&tecido, &item)
            }
            AgentAction::OpenVenda { id } => self.agent_open_venda(id),
            AgentAction::FilterSalesHistory { inicio, fim } => {
                self.venda_historico_inicio = inicio;
                self.venda_historico_fim = fim;
                self.reload_vendas_historico();
                self.section = Section::Vendas;
                self.vendas_screen = VendasScreen::Historico;
                String::from("Historico filtrado.")
            }
            AgentAction::SelectPrinter { printer } => {
                if let Some(index) = self.printers.iter().position(|item| item == &printer) {
                    self.printer_option = index;
                    self.selected_printer = Some(printer.clone());
                    if let Some(pool) = &self.db_pool
                        && let Err(error) = self.db_runtime.block_on(db::set_config(
                            pool,
                            "receipt_printer",
                            &printer,
                        ))
                    {
                        return format!("Erro ao salvar impressora: {error}");
                    }
                    format!("Impressora selecionada: {printer}.")
                } else {
                    format!("Impressora nao encontrada: {printer}.")
                }
            }
            AgentAction::AddVendaItem {
                tecido,
                item,
                preco,
                quantidade,
            } => self.agent_add_venda_item(&tecido, &item, &preco, &quantidade),
            AgentAction::AddPedidoItem {
                tecido,
                item,
                preco,
                quantidade,
            } => self.agent_add_pedido_item(&tecido, &item, &preco, &quantidade),
        }
    }

    fn agent_create_cor(&mut self, nome: &str, hex: &str) -> String {
        if parse_hex_color(hex).is_none() {
            return String::from("Hex invalido. Use #RRGGBB.");
        }
        let form = CorForm {
            nome: nome.to_string(),
            hex: hex.to_string(),
            ..CorForm::default()
        };
        let sku = form.sku(&self.cores, None);
        if let Some(pool) = &self.db_pool
            && let Err(error) = self
                .db_runtime
                .block_on(db::insert_cor(pool, nome, &sku, hex))
        {
            return format!("Erro ao cadastrar cor: {error}");
        }
        self.reload_cores();
        format!("Cor cadastrada: {nome} ({sku}).")
    }

    fn agent_create_estampa(&mut self, nome: &str) -> String {
        let form = EstampaForm {
            nome: nome.to_string(),
            ..EstampaForm::default()
        };
        let sku = form.sku(&self.estampas, None);
        if let Some(pool) = &self.db_pool
            && let Err(error) = self
                .db_runtime
                .block_on(db::insert_estampa(pool, nome, &sku))
        {
            return format!("Erro ao cadastrar estampa: {error}");
        }
        self.reload_estampas();
        format!("Estampa cadastrada: {nome} ({sku}).")
    }

    #[allow(clippy::too_many_arguments)]
    fn agent_create_tecido(
        &mut self,
        nome: &str,
        composicao: &str,
        largura: &str,
        tipo: Option<&str>,
        rendimento: Option<&str>,
        gramatura_linear: Option<&str>,
        gramatura_m2: Option<&str>,
    ) -> String {
        let form = TecidoForm {
            nome: nome.to_string(),
            composicao: composicao.to_string(),
            largura: largura.to_string(),
            tipo: tipo
                .map(|value| SelectValue::from_value(value, TIPO_OPTIONS))
                .unwrap_or_default(),
            rendimento: rendimento.unwrap_or_default().to_string(),
            gramatura_linear: gramatura_linear.unwrap_or_default().to_string(),
            gramatura_m2: gramatura_m2.unwrap_or_default().to_string(),
            ..TecidoForm::default()
        };
        if !form.is_valid() {
            return String::from(
                "Dados obrigatorios invalidos para tecido: nome, composicao e largura.",
            );
        }
        let sku = form.sku(&self.tecidos, None);
        if let Some(pool) = &self.db_pool
            && let Err(error) = self
                .db_runtime
                .block_on(db::insert_tecido(pool, &form, &sku, None))
        {
            return format!("Erro ao cadastrar tecido: {error}");
        }
        self.reload_tecidos();
        format!("Tecido cadastrado: {nome} ({sku}).")
    }

    fn agent_create_vinculo(&mut self, tecido_name: &str, item_name: &str) -> String {
        let Some(tecido) = find_by_name(&self.tecidos, tecido_name, |tecido| &tecido.nome).cloned()
        else {
            return format!("Tecido nao encontrado: {tecido_name}.");
        };
        self.vinculo_tecido_option = self
            .tecidos
            .iter()
            .position(|item| item.id == tecido.id)
            .unwrap_or(0);
        self.load_vinculos(tecido.id);
        self.selected_vinculo_cores = self.vinculos.iter().map(|vinculo| vinculo.cor_id).collect();

        if tecido.tipo == "Estampado" {
            let Some(estampa) = find_by_name(&self.estampas, item_name, |item| &item.nome) else {
                return format!("Estampa nao encontrada: {item_name}.");
            };
            if !self.selected_vinculo_cores.contains(&estampa.id) {
                self.selected_vinculo_cores.push(estampa.id);
            }
        } else {
            let Some(cor) = find_by_name(&self.cores, item_name, |item| &item.nome) else {
                return format!("Cor nao encontrada: {item_name}.");
            };
            if !self.selected_vinculo_cores.contains(&cor.id) {
                self.selected_vinculo_cores.push(cor.id);
            }
        }
        self.salvar_vinculos();
        format!(
            "Vinculo criado/confirmado: {} + {}.",
            tecido.nome, item_name
        )
    }

    fn agent_open_venda(&mut self, id: i64) -> String {
        self.reload_vendas_historico();
        let Some(index) = self
            .vendas_historico
            .iter()
            .position(|venda| venda.id == id)
        else {
            return format!("Venda #{id} nao encontrada no periodo carregado.");
        };
        self.venda_historico_option = index;
        self.section = Section::Vendas;
        self.vendas_screen = VendasScreen::Historico;
        self.open_edit_venda();
        format!("Venda #{id} aberta para edicao.")
    }

    fn agent_add_venda_item(
        &mut self,
        tecido: &str,
        item: &str,
        preco: &str,
        quantidade: &str,
    ) -> String {
        let Some(venda_item) = build_venda_item(tecido, item, preco, quantidade) else {
            return String::from("Preco ou quantidade invalidos para lancamento de venda.");
        };
        self.venda_itens.push(venda_item);
        self.venda_item_option = self.venda_itens.len().saturating_sub(1);
        self.section = Section::Vendas;
        self.vendas_screen = VendasScreen::Lancamento;
        format!("Lancamento adicionado a venda: {tecido} - {item}.")
    }

    fn agent_add_pedido_item(
        &mut self,
        tecido: &str,
        item: &str,
        preco: &str,
        quantidade: &str,
    ) -> String {
        let Some(pedido_item) = build_venda_item(tecido, item, preco, quantidade) else {
            return String::from("Preco ou quantidade invalidos para lancamento de pedido.");
        };
        self.pedido_itens.push(pedido_item);
        self.pedido_item_option = self.pedido_itens.len().saturating_sub(1);
        self.section = Section::Pedidos;
        self.pedidos_screen = crate::models::PedidosScreen::Lancamento;
        format!("Lancamento adicionado ao pedido: {tecido} - {item}.")
    }

    fn dashboard_agent_context(&self) -> String {
        self.agent_context()
    }
}

fn parse_agent_action(message: &str) -> Option<AgentAction> {
    let normalized = normalize(message);
    if normalized.contains("cad") && normalized.contains("cor") {
        let hex = extract_hex(message)?;
        let nome =
            cleanup_name_after_keywords(message, &["cor", "chamada", "nome"]).replace(&hex, "");
        return Some(AgentAction::CreateCor {
            nome: nome.trim().to_string(),
            hex,
        });
    }
    if normalized.contains("cad") && normalized.contains("estampa") {
        return Some(AgentAction::CreateEstampa {
            nome: cleanup_name_after_keywords(message, &["estampa", "chamada", "nome"]),
        });
    }
    if normalized.contains("cad") && normalized.contains("tecido") {
        let nome = value_after(message, "tecido")
            .unwrap_or_else(|| cleanup_name_after_keywords(message, &["tecido"]));
        let composicao =
            value_after(message, "composicao").or_else(|| value_after(message, "composição"))?;
        let largura = value_after(message, "largura")?;
        let tipo = if normalized.contains("estampado") {
            Some(String::from("Estampado"))
        } else if normalized.contains("liso") {
            Some(String::from("Liso"))
        } else {
            None
        };
        return Some(AgentAction::CreateTecido {
            nome,
            composicao,
            largura,
            tipo,
            rendimento: value_after(message, "rendimento"),
            gramatura_linear: value_after(message, "gramatura linear")
                .or_else(|| value_after(message, "linear")),
            gramatura_m2: value_after(message, "gramatura m2")
                .or_else(|| value_after(message, "gramatura m²"))
                .or_else(|| value_after(message, "m2")),
        });
    }
    if normalized.contains("vincul") {
        let (tecido, item) = split_between(message)?;
        return Some(AgentAction::CreateVinculo { tecido, item });
    }
    if normalized.contains("venda") && normalized.contains('#') {
        let id = message
            .split('#')
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()?;
        return Some(AgentAction::OpenVenda { id });
    }
    if normalized.contains("historico") || normalized.contains("histórico") {
        let dates = extract_dates(message);
        if dates.len() >= 2 {
            return Some(AgentAction::FilterSalesHistory {
                inicio: dates[0].clone(),
                fim: dates[1].clone(),
            });
        }
    }
    if normalized.contains("impressora")
        && (normalized.contains("selecion") || normalized.contains("configur"))
    {
        let printer =
            value_after(message, "impressora").or_else(|| value_after(message, "printer"))?;
        return Some(AgentAction::SelectPrinter { printer });
    }
    None
}

enum DraftStep {
    Ask(String),
    Confirm(AgentAction),
}

fn draft_from_context(app: &App, message: &str) -> Option<AgentDraft> {
    let normalized = normalize(message);
    let wants_create = is_creation_intent(&normalized);

    if !wants_create {
        return None;
    }

    if normalized.contains("pedido") {
        return Some(AgentDraft::AddPedidoItem {
            tecido: value_after(message, "tecido"),
            item: value_after(message, "item")
                .or_else(|| value_after(message, "cor"))
                .or_else(|| value_after(message, "estampa")),
            preco: value_after(message, "preco").or_else(|| value_after(message, "preço")),
            quantidade: value_after(message, "quantidade").or_else(|| value_after(message, "qtd")),
        });
    }

    if normalized.contains("venda") {
        return Some(AgentDraft::AddVendaItem {
            tecido: value_after(message, "tecido"),
            item: value_after(message, "item")
                .or_else(|| value_after(message, "cor"))
                .or_else(|| value_after(message, "estampa")),
            preco: value_after(message, "preco").or_else(|| value_after(message, "preço")),
            quantidade: value_after(message, "quantidade").or_else(|| value_after(message, "qtd")),
        });
    }

    if normalized.contains("impressora") {
        return Some(AgentDraft::SelectPrinter {
            printer: value_after(message, "impressora")
                .or_else(|| value_after(message, "printer"))
                .or_else(|| name_after_create_word(message, &["impressora"])),
        });
    }

    if app.section == Section::Pedidos {
        return Some(AgentDraft::AddPedidoItem {
            tecido: value_after(message, "tecido"),
            item: value_after(message, "item")
                .or_else(|| value_after(message, "cor"))
                .or_else(|| value_after(message, "estampa")),
            preco: value_after(message, "preco").or_else(|| value_after(message, "preço")),
            quantidade: value_after(message, "quantidade").or_else(|| value_after(message, "qtd")),
        });
    }

    if app.section == Section::Vendas {
        return Some(AgentDraft::AddVendaItem {
            tecido: value_after(message, "tecido"),
            item: value_after(message, "item")
                .or_else(|| value_after(message, "cor"))
                .or_else(|| value_after(message, "estampa")),
            preco: value_after(message, "preco").or_else(|| value_after(message, "preço")),
            quantidade: value_after(message, "quantidade").or_else(|| value_after(message, "qtd")),
        });
    }

    if normalized.contains("tecido")
        || matches!(
            (app.section, app.dados_screen, app.dados_option),
            (Section::Dados, DadosScreen::CadastrarTecido, _)
                | (Section::Dados, _, DadosOption::Tecido)
        )
    {
        let mut draft = AgentDraft::CreateTecido {
            nome: quoted_text(message)
                .or_else(|| value_after(message, "chamado"))
                .or_else(|| value_after(message, "chamada"))
                .or_else(|| value_after(message, "tecido"))
                .or_else(|| value_after(message, "nome"))
                .or_else(|| name_after_create_word(message, &["tecido"])),
            composicao: value_after(message, "composicao")
                .or_else(|| value_after(message, "composição")),
            largura: value_after(message, "largura"),
            tipo: infer_tipo(message),
            rendimento: value_after(message, "rendimento"),
            gramatura_linear: value_after(message, "gramatura linear")
                .or_else(|| value_after(message, "linear")),
            gramatura_m2: value_after(message, "gramatura m2")
                .or_else(|| value_after(message, "gramatura m²"))
                .or_else(|| value_after(message, "m2")),
        };
        merge_draft_from_message(&mut draft, message);
        return Some(draft);
    }

    if normalized.contains("cor")
        || matches!(
            (app.section, app.dados_screen, app.dados_option),
            (Section::Dados, DadosScreen::CadastrarCor, _)
                | (Section::Dados, _, DadosOption::Cores)
        )
    {
        let hex = extract_hex(message);
        let mut draft = AgentDraft::CreateCor {
            nome: quoted_text(message)
                .or_else(|| value_after(message, "chamada"))
                .or_else(|| value_after(message, "chamado"))
                .or_else(|| value_after(message, "cor"))
                .or_else(|| value_after(message, "nome"))
                .or_else(|| name_after_create_word(message, &["cor"]))
                .map(|nome| remove_hex_from_name(&nome, hex.as_deref())),
            hex,
        };
        merge_draft_from_message(&mut draft, message);
        return Some(draft);
    }

    if normalized.contains("estampa")
        || matches!(
            (app.section, app.dados_screen, app.dados_option),
            (Section::Dados, DadosScreen::CadastrarEstampa, _)
                | (Section::Dados, _, DadosOption::Estampas)
        )
    {
        let mut draft = AgentDraft::CreateEstampa {
            nome: quoted_text(message)
                .or_else(|| value_after(message, "chamada"))
                .or_else(|| value_after(message, "chamado"))
                .or_else(|| value_after(message, "estampa"))
                .or_else(|| value_after(message, "nome"))
                .or_else(|| name_after_create_word(message, &["estampa"])),
        };
        merge_draft_from_message(&mut draft, message);
        return Some(draft);
    }

    if normalized.contains("vincul")
        || matches!(
            (app.section, app.dados_screen, app.dados_option),
            (Section::Dados, DadosScreen::VinculosMenu, _)
                | (
                    Section::Dados,
                    DadosScreen::VinculosSelecionarTecidoCriar,
                    _
                )
                | (Section::Dados, DadosScreen::VinculosSelecionarCores, _)
                | (Section::Dados, _, DadosOption::Vinculos)
        )
    {
        let (tecido, item) = split_between(message).unwrap_or_else(|| {
            (
                value_after(message, "tecido").unwrap_or_default(),
                value_after(message, "item")
                    .or_else(|| value_after(message, "cor"))
                    .or_else(|| value_after(message, "estampa"))
                    .unwrap_or_default(),
            )
        });
        return Some(AgentDraft::CreateVinculo {
            tecido: non_empty(tecido),
            item: non_empty(item),
        });
    }

    if app.section == Section::Configuracoes {
        return Some(AgentDraft::SelectPrinter {
            printer: value_after(message, "impressora")
                .or_else(|| value_after(message, "printer"))
                .or_else(|| name_after_create_word(message, &["impressora"])),
        });
    }

    None
}

fn merge_draft_from_message(draft: &mut AgentDraft, message: &str) {
    if message.trim().is_empty() {
        return;
    }
    match draft {
        AgentDraft::CreateTecido {
            nome,
            composicao,
            largura,
            tipo,
            rendimento,
            gramatura_linear,
            gramatura_m2,
        } => {
            if nome.is_none() {
                *nome = quoted_text(message)
                    .or_else(|| value_after(message, "chamado"))
                    .or_else(|| value_after(message, "chamada"))
                    .or_else(|| value_after(message, "tecido"))
                    .or_else(|| value_after(message, "nome"))
                    .or_else(|| free_answer(message));
                return;
            }
            if composicao.is_none() {
                *composicao = value_after(message, "composicao")
                    .or_else(|| value_after(message, "composição"))
                    .or_else(|| free_answer(message));
                return;
            }
            if largura.is_none() {
                *largura = value_after(message, "largura")
                    .or_else(|| extract_largura_token(message))
                    .or_else(|| free_answer(message));
            }
            if rendimento.is_none() {
                *rendimento = value_after(message, "rendimento");
            }
            if gramatura_linear.is_none() {
                *gramatura_linear = value_after(message, "gramatura linear")
                    .or_else(|| value_after(message, "linear"));
            }
            if gramatura_m2.is_none() {
                *gramatura_m2 = value_after(message, "gramatura m2")
                    .or_else(|| value_after(message, "gramatura m²"))
                    .or_else(|| value_after(message, "m2"));
            }
            if tipo.is_none() {
                *tipo = infer_tipo(message);
            }
        }
        AgentDraft::CreateCor { nome, hex } => {
            if nome.is_none() {
                *nome = quoted_text(message)
                    .or_else(|| value_after(message, "chamada"))
                    .or_else(|| value_after(message, "chamado"))
                    .or_else(|| value_after(message, "cor"))
                    .or_else(|| value_after(message, "nome"))
                    .or_else(|| free_answer(message))
                    .map(|nome| remove_hex_from_name(&nome, hex.as_deref()));
                return;
            }
            if hex.is_none() {
                *hex = extract_hex(message).or_else(|| free_answer(message));
            }
        }
        AgentDraft::CreateEstampa { nome } => {
            if nome.is_none() {
                *nome = quoted_text(message)
                    .or_else(|| value_after(message, "chamada"))
                    .or_else(|| value_after(message, "chamado"))
                    .or_else(|| value_after(message, "estampa"))
                    .or_else(|| value_after(message, "nome"))
                    .or_else(|| free_answer(message));
            }
        }
        AgentDraft::CreateVinculo { tecido, item } => {
            if let Some((parsed_tecido, parsed_item)) = split_between(message) {
                if tecido.is_none() {
                    *tecido = Some(parsed_tecido);
                }
                if item.is_none() {
                    *item = Some(parsed_item);
                }
            }
            if tecido.is_none() {
                *tecido = value_after(message, "tecido").or_else(|| free_answer(message));
            } else if item.is_none() {
                *item = value_after(message, "item")
                    .or_else(|| value_after(message, "cor"))
                    .or_else(|| value_after(message, "estampa"))
                    .or_else(|| free_answer(message));
            }
        }
        AgentDraft::SelectPrinter { printer } => {
            if printer.is_none() {
                *printer = value_after(message, "impressora")
                    .or_else(|| value_after(message, "printer"))
                    .or_else(|| free_answer(message));
            }
        }
        AgentDraft::AddVendaItem {
            tecido,
            item,
            preco,
            quantidade,
        }
        | AgentDraft::AddPedidoItem {
            tecido,
            item,
            preco,
            quantidade,
        } => merge_item_draft(message, tecido, item, preco, quantidade),
    }
}

fn draft_next_step(draft: &AgentDraft) -> DraftStep {
    match draft {
        AgentDraft::CreateTecido {
            nome,
            composicao,
            largura,
            tipo,
            rendimento,
            gramatura_linear,
            gramatura_m2,
        } => {
            let Some(nome) = clean_option(nome) else {
                return DraftStep::Ask(String::from("Qual e o nome do tecido?"));
            };
            let Some(composicao) = clean_option(composicao) else {
                return DraftStep::Ask(format!("Qual e a composicao do tecido '{nome}'?"));
            };
            let Some(largura) = clean_option(largura) else {
                return DraftStep::Ask(format!("Qual e a largura do tecido '{nome}'? Ex: 1.50m"));
            };
            DraftStep::Confirm(AgentAction::CreateTecido {
                nome,
                composicao,
                largura,
                tipo: clean_option(tipo),
                rendimento: clean_option(rendimento),
                gramatura_linear: clean_option(gramatura_linear),
                gramatura_m2: clean_option(gramatura_m2),
            })
        }
        AgentDraft::CreateCor { nome, hex } => {
            let Some(nome) = clean_option(nome) else {
                return DraftStep::Ask(String::from("Qual e o nome da cor?"));
            };
            let Some(hex) = clean_option(hex) else {
                return DraftStep::Ask(format!(
                    "Qual e o hexadecimal da cor '{nome}'? Ex: #FF0000"
                ));
            };
            if parse_hex_color(&hex).is_none() {
                return DraftStep::Ask(String::from("Hex invalido. Envie no formato #RRGGBB."));
            }
            DraftStep::Confirm(AgentAction::CreateCor { nome, hex })
        }
        AgentDraft::CreateEstampa { nome } => {
            let Some(nome) = clean_option(nome) else {
                return DraftStep::Ask(String::from("Qual e o nome da estampa?"));
            };
            DraftStep::Confirm(AgentAction::CreateEstampa { nome })
        }
        AgentDraft::CreateVinculo { tecido, item } => {
            let Some(tecido) = clean_option(tecido) else {
                return DraftStep::Ask(String::from("Qual tecido deve receber o vinculo?"));
            };
            let Some(item) = clean_option(item) else {
                return DraftStep::Ask(format!(
                    "Qual cor ou estampa deseja vincular ao tecido '{tecido}'?"
                ));
            };
            DraftStep::Confirm(AgentAction::CreateVinculo { tecido, item })
        }
        AgentDraft::SelectPrinter { printer } => {
            let Some(printer) = clean_option(printer) else {
                return DraftStep::Ask(String::from("Qual impressora deseja selecionar?"));
            };
            DraftStep::Confirm(AgentAction::SelectPrinter { printer })
        }
        AgentDraft::AddVendaItem {
            tecido,
            item,
            preco,
            quantidade,
        } => item_draft_next_step(tecido, item, preco, quantidade, true),
        AgentDraft::AddPedidoItem {
            tecido,
            item,
            preco,
            quantidade,
        } => item_draft_next_step(tecido, item, preco, quantidade, false),
    }
}

fn merge_item_draft(
    message: &str,
    tecido: &mut Option<String>,
    item: &mut Option<String>,
    preco: &mut Option<String>,
    quantidade: &mut Option<String>,
) {
    if tecido.is_none() {
        *tecido = labeled_value_after(message, "tecido").or_else(|| free_answer(message));
    } else if item.is_none() {
        *item = labeled_value_after(message, "item")
            .or_else(|| labeled_value_after(message, "cor"))
            .or_else(|| labeled_value_after(message, "estampa"))
            .or_else(|| free_answer(message));
    } else if preco.is_none() {
        *preco = labeled_value_after(message, "preco")
            .or_else(|| labeled_value_after(message, "preço"))
            .or_else(|| free_answer(message));
    } else if quantidade.is_none() {
        *quantidade = labeled_value_after(message, "quantidade")
            .or_else(|| labeled_value_after(message, "qtd"))
            .or_else(|| free_answer(message));
    }
}

fn item_draft_next_step(
    tecido: &Option<String>,
    item: &Option<String>,
    preco: &Option<String>,
    quantidade: &Option<String>,
    is_venda: bool,
) -> DraftStep {
    let Some(tecido) = clean_option(tecido) else {
        return DraftStep::Ask(String::from("Qual tecido deseja lancar?"));
    };
    let Some(item) = clean_option(item) else {
        return DraftStep::Ask(format!(
            "Qual cor ou estampa deseja usar para o tecido '{tecido}'?"
        ));
    };
    let Some(preco) = clean_option(preco) else {
        return DraftStep::Ask(format!("Qual preco unitario para '{tecido} - {item}'?"));
    };
    if parse_number(&preco).filter(|value| *value > 0.0).is_none() {
        return DraftStep::Ask(String::from(
            "Preco invalido. Informe um valor maior que zero.",
        ));
    }
    let Some(quantidade) = clean_option(quantidade) else {
        return DraftStep::Ask(format!("Qual quantidade para '{tecido} - {item}'?"));
    };
    if parse_number(&quantidade)
        .filter(|value| *value > 0.0)
        .is_none()
    {
        return DraftStep::Ask(String::from(
            "Quantidade invalida. Informe um valor maior que zero.",
        ));
    }
    if is_venda {
        DraftStep::Confirm(AgentAction::AddVendaItem {
            tecido,
            item,
            preco,
            quantidade,
        })
    } else {
        DraftStep::Confirm(AgentAction::AddPedidoItem {
            tecido,
            item,
            preco,
            quantidade,
        })
    }
}

fn describe_action(action: &AgentAction) -> String {
    match action {
        AgentAction::CreateTecido {
            nome,
            composicao,
            largura,
            tipo,
            rendimento,
            gramatura_linear,
            gramatura_m2,
        } => {
            let calc = tecido_calculation_summary(
                largura,
                rendimento.as_deref(),
                gramatura_linear.as_deref(),
                gramatura_m2.as_deref(),
            );
            format!(
                "Cadastrar tecido '{nome}', composicao '{composicao}', largura '{largura}', tipo '{}'.{}",
                tipo.as_deref().unwrap_or("Selecione"),
                calc.map(|value| format!("\n{value}")).unwrap_or_default()
            )
        }
        AgentAction::CreateCor { nome, hex } => format!("Cadastrar cor '{nome}' com hex {hex}."),
        AgentAction::CreateEstampa { nome } => format!("Cadastrar estampa '{nome}'."),
        AgentAction::CreateVinculo { tecido, item } => {
            format!("Criar vinculo entre '{tecido}' e '{item}'.")
        }
        AgentAction::OpenVenda { id } => format!("Abrir venda #{id} para edicao."),
        AgentAction::FilterSalesHistory { inicio, fim } => {
            format!("Filtrar historico de vendas de {inicio} ate {fim}.")
        }
        AgentAction::SelectPrinter { printer } => format!("Selecionar impressora '{printer}'."),
        AgentAction::AddVendaItem {
            tecido,
            item,
            preco,
            quantidade,
        } => format!(
            "Adicionar item a venda: {tecido} - {item}, preco {preco}, quantidade {quantidade}."
        ),
        AgentAction::AddPedidoItem {
            tecido,
            item,
            preco,
            quantidade,
        } => format!(
            "Adicionar item ao pedido: {tecido} - {item}, preco {preco}, quantidade {quantidade}."
        ),
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn is_creation_intent(normalized: &str) -> bool {
    normalized.contains("criar")
        || normalized.contains("crie")
        || normalized.contains("cad")
        || normalized.contains("novo")
        || normalized.contains("nova")
        || normalized.contains("adicionar")
        || normalized.contains("incluir")
        || normalized.contains("configur")
}

fn format_named_list<I>(title: &str, values: I) -> String
where
    I: Iterator<Item = String>,
{
    let items = values.collect::<Vec<_>>();
    if items.is_empty() {
        return format!("{title}: nenhum registro.");
    }
    format!("{title}:\n- {}", items.join("\n- "))
}

fn format_money(value: f64) -> String {
    format!("{value:.2}").replace('.', ",")
}

fn build_venda_item(tecido: &str, item: &str, preco: &str, quantidade: &str) -> Option<VendaItem> {
    let preco_unitario = parse_number(preco).filter(|value| *value > 0.0)?;
    let quantidade = parse_number(quantidade).filter(|value| *value > 0.0)?;
    Some(VendaItem {
        descricao: format!("{tecido} - {item}"),
        quantidade,
        preco_unitario,
        estoque_tecido_id: None,
        estoque_item_id: None,
        estoque_usa_estampas: false,
    })
}

fn tecido_calculation_summary(
    largura: &str,
    rendimento: Option<&str>,
    gramatura_linear: Option<&str>,
    gramatura_m2: Option<&str>,
) -> Option<String> {
    let form = TecidoForm {
        largura: largura.to_string(),
        rendimento: rendimento.unwrap_or_default().to_string(),
        gramatura_linear: gramatura_linear.unwrap_or_default().to_string(),
        gramatura_m2: gramatura_m2.unwrap_or_default().to_string(),
        ..TecidoForm::default()
    };
    let calculated = form.calculated_values();
    if calculated.rendimento.is_none()
        && calculated.gramatura_linear.is_none()
        && calculated.gramatura_m2.is_none()
    {
        return None;
    }
    Some(format!(
        "Calculo: rendimento {} m/kg, gramatura linear {} g/m, gramatura m2 {} g/m2.",
        optional_number(calculated.rendimento),
        optional_number(calculated.gramatura_linear),
        optional_number(calculated.gramatura_m2)
    ))
}

fn optional_number(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}").replace('.', ","))
        .unwrap_or_else(|| String::from("-"))
}

fn extract_hex(value: &str) -> Option<String> {
    value
        .split_whitespace()
        .find(|part| part.starts_with('#') && part.len() == 7)
        .map(|part| {
            part.trim_matches(|ch: char| ch == '.' || ch == ',')
                .to_string()
        })
}

fn extract_dates(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(|part| part.trim_matches(|ch: char| ch == '.' || ch == ',' || ch == ';'))
        .filter(|part| {
            part.len() == 10
                && part.chars().enumerate().all(|(index, ch)| {
                    if index == 4 || index == 7 {
                        ch == '-'
                    } else {
                        ch.is_ascii_digit()
                    }
                })
        })
        .map(ToString::to_string)
        .collect()
}

fn cleanup_name_after_keywords(value: &str, keywords: &[&str]) -> String {
    let mut cleaned = value.to_string();
    for keyword in keywords {
        cleaned = cleaned.replace(keyword, "");
        cleaned = cleaned.replace(&capitalize(keyword), "");
    }
    cleaned
        .replace("cadastre", "")
        .replace("cadastrar", "")
        .replace("quero", "")
        .replace("Quero", "")
        .replace("chamado", "")
        .replace("chamada", "")
        .replace("com hex", "")
        .replace("hex", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn value_after(value: &str, marker: &str) -> Option<String> {
    let lower = value.to_lowercase();
    let index = lower.find(marker)? + marker.len();
    let tail = value[index..].trim_start_matches([':', ' ', '=']);
    let end = tail.find([',', ';']).unwrap_or(tail.len());
    Some(tail[..end].trim().to_string()).filter(|item| !item.is_empty())
}

fn labeled_value_after(value: &str, marker: &str) -> Option<String> {
    let lower = value.to_lowercase();
    let marker = marker.to_lowercase();
    if lower.starts_with(&(marker.clone() + ":"))
        || lower.starts_with(&(marker.clone() + "="))
        || lower.starts_with(&(marker.clone() + " "))
    {
        value_after(value, &marker)
    } else {
        None
    }
}

fn quoted_text(value: &str) -> Option<String> {
    let start = value.find('"').or_else(|| value.find('\''))?;
    let quote = value[start..].chars().next()?;
    let tail = &value[start + quote.len_utf8()..];
    let end = tail.find(quote)?;
    non_empty(tail[..end].to_string())
}

fn extract_largura_token(value: &str) -> Option<String> {
    value
        .split_whitespace()
        .map(|part| part.trim_matches(|ch: char| ch == ',' || ch == ';' || ch == '.'))
        .find(|part| {
            let normalized = part.replace(',', ".");
            normalized.ends_with('m')
                && normalized[..normalized.len().saturating_sub(1)]
                    .parse::<f64>()
                    .is_ok()
        })
        .map(ToString::to_string)
}

fn name_after_create_word(value: &str, remove_words: &[&str]) -> Option<String> {
    let normalized = normalize(value);
    if !(normalized.contains("criar")
        || normalized.contains("cad")
        || normalized.contains("novo")
        || normalized.contains("nova")
        || normalized.contains("adicionar")
        || normalized.contains("incluir"))
    {
        return None;
    }
    let mut cleaned = cleanup_name_after_keywords(value, remove_words);
    for word in [
        "criar",
        "crie",
        "novo",
        "nova",
        "adicionar",
        "incluir",
        "configurar",
        "configure",
    ] {
        cleaned = cleaned.replace(word, "");
        cleaned = cleaned.replace(&capitalize(word), "");
    }
    non_empty(cleaned)
}

fn free_answer(value: &str) -> Option<String> {
    let normalized = normalize(value);
    if normalized.contains("criar")
        || normalized.contains("cad")
        || normalized.contains("novo")
        || normalized.contains("nova")
        || normalized.contains("adicionar")
        || normalized.contains("incluir")
        || normalized.contains("configur")
    {
        return None;
    }
    non_empty(value.trim().to_string())
}

fn infer_tipo(value: &str) -> Option<String> {
    let normalized = normalize(value);
    if normalized.contains("estampado") || normalized.contains("estampada") {
        Some(String::from("Estampado"))
    } else if normalized.contains("liso") || normalized.contains("lisa") {
        Some(String::from("Liso"))
    } else {
        None
    }
}

fn clean_option(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn remove_hex_from_name(value: &str, hex: Option<&str>) -> String {
    hex.map(|hex| value.replace(hex, ""))
        .unwrap_or_else(|| value.to_string())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_between(value: &str) -> Option<(String, String)> {
    let lower = value.to_lowercase();
    let start = lower.find("entre").map(|index| index + 5).unwrap_or(0);
    let tail = value[start..].trim();
    let separator = tail.find(" e ").or_else(|| tail.find(" com "))?;
    Some((
        tail[..separator].trim().to_string(),
        tail[separator + 3..].trim().to_string(),
    ))
}

fn find_by_name<'a, T, F>(items: &'a [T], name: &str, get_name: F) -> Option<&'a T>
where
    F: Fn(&T) -> &String,
{
    let target = normalize(name);
    items
        .iter()
        .find(|item| normalize(get_name(item)) == target)
        .or_else(|| {
            items
                .iter()
                .find(|item| normalize(get_name(item)).contains(&target))
        })
}

#[cfg(test)]
mod tests {
    use crate::{app::App, db};
    use ratatui_image::picker::Picker;
    use sqlx::Row;
    use tokio::runtime::Runtime;

    #[test]
    fn guided_tecido_flow_calculates_and_persists() {
        let (mut app, pool) = test_app();
        let nome = "Teste Agente Viscolinho";

        cleanup_named_test_data(&app, &[nome], &[], &[]);

        app.submit_dashboard_agent_message(&format!("Quero criar um tecido chamado \"{nome}\""));
        assert!(
            app.chat
                .messages
                .last()
                .expect("pergunta composicao")
                .text
                .contains("composicao")
        );

        assert!(app.handle_pending_agent_draft("100% Viscose"));
        assert!(
            app.chat
                .messages
                .last()
                .expect("pergunta largura")
                .text
                .contains("largura")
        );

        assert!(app.handle_pending_agent_draft("1,50m rendimento 3"));
        let confirmation = &app.chat.messages.last().expect("confirmacao").text;
        assert!(confirmation.contains("Calculo:"));
        assert!(confirmation.contains("333,33"));
        assert!(confirmation.contains("222,22"));

        assert!(app.handle_pending_agent_confirmation("sim"));
        let row = app
            .db_runtime
            .block_on(
                sqlx::query(
                    r#"
                    SELECT
                        composicao,
                        largura_m::float8 AS largura_m,
                        rendimento_m_kg::float8 AS rendimento_m_kg,
                        gramatura_linear_g_m,
                        gramatura_g_m2
                    FROM tecidos
                    WHERE nome = $1
                    "#,
                )
                .bind(nome)
                .fetch_one(&pool),
            )
            .expect("tecido cadastrado");

        let composicao: String = row.get("composicao");
        let largura_m: f64 = row.get("largura_m");
        let rendimento_m_kg: Option<f64> = row.get("rendimento_m_kg");
        let gramatura_linear_g_m: Option<i32> = row.get("gramatura_linear_g_m");
        let gramatura_g_m2: Option<i32> = row.get("gramatura_g_m2");

        assert_eq!(composicao, "100% Viscose");
        assert!((largura_m - 1.5).abs() < 0.001);
        assert_eq!(
            rendimento_m_kg.map(|value| (value * 100.0).round() / 100.0),
            Some(3.0)
        );
        assert_eq!(gramatura_linear_g_m, Some(330));
        assert_eq!(gramatura_g_m2, Some(220));
    }

    #[test]
    fn guided_all_capability_flows_work() {
        let (mut app, pool) = test_app();
        let tecido = "Teste Agente Tecido Geral";
        let cor = "Teste Agente Azul";
        let estampa = "Teste Agente Floral";

        cleanup_named_test_data(&app, &[tecido], &[cor], &[estampa]);

        create_tecido_via_agent(&mut app, tecido);
        assert_db_tecido(&app, &pool, tecido, "100% Algodao");

        create_cor_via_agent(&mut app, cor, "#224466");
        assert_db_cor(&app, &pool, cor, "#224466");

        create_estampa_via_agent(&mut app, estampa);
        assert_db_estampa(&app, &pool, estampa);

        app.submit_dashboard_agent_message(&format!("criar vinculo entre {tecido} e {cor}"));
        assert_last_contains(&app, "Criar vinculo");
        assert!(app.handle_pending_agent_confirmation("sim"));
        assert_last_contains(&app, "Vinculo criado/confirmado");
        assert_db_vinculo_cor(&app, &pool, tecido, cor);

        app.submit_dashboard_agent_message("criar venda");
        assert_last_contains(&app, "Qual tecido");
        assert!(app.handle_pending_agent_draft(tecido));
        assert_last_contains(&app, "Qual cor ou estampa");
        assert!(app.handle_pending_agent_draft(cor));
        assert_last_contains(&app, "Qual preco");
        assert!(app.handle_pending_agent_draft("12,50"));
        assert_last_contains(&app, "Qual quantidade");
        assert!(app.handle_pending_agent_draft("2"));
        assert_last_contains(&app, "Adicionar item a venda");
        assert!(app.handle_pending_agent_confirmation("sim"));
        assert_eq!(app.venda_itens.len(), 1);
        assert_eq!(app.venda_itens[0].descricao, format!("{tecido} - {cor}"));

        app.submit_dashboard_agent_message("criar pedido");
        assert_last_contains(&app, "Qual tecido");
        assert!(app.handle_pending_agent_draft(tecido));
        assert_last_contains(&app, "Qual cor ou estampa");
        assert!(app.handle_pending_agent_draft(cor));
        assert_last_contains(&app, "Qual preco");
        assert!(app.handle_pending_agent_draft("20"));
        assert_last_contains(&app, "Qual quantidade");
        assert!(app.handle_pending_agent_draft("3"));
        assert_last_contains(&app, "Adicionar item ao pedido");
        assert!(app.handle_pending_agent_confirmation("sim"));
        assert_eq!(app.pedido_itens.len(), 1);
        assert_eq!(app.pedido_itens[0].descricao, format!("{tecido} - {cor}"));

        app.submit_dashboard_agent_message("configurar impressora");
        assert_last_contains(&app, "Qual impressora");
        assert!(app.handle_pending_agent_draft("Impressora Teste Inexistente"));
        assert_last_contains(&app, "Selecionar impressora");
        assert!(app.handle_pending_agent_confirmation("sim"));
        assert_last_contains(&app, "Impressora nao encontrada");
    }

    fn test_app() -> (App, sqlx::PgPool) {
        dotenvy::dotenv().ok();
        let runtime = Runtime::new().expect("runtime");
        let pool = runtime.block_on(db::connect()).expect("conectar banco");
        runtime
            .block_on(db::ensure_configuracoes_table(&pool))
            .expect("configuracoes");
        runtime
            .block_on(db::ensure_fornecedores_table(&pool))
            .expect("fornecedores");
        runtime
            .block_on(db::ensure_estampas_tables(&pool))
            .expect("estampas");
        runtime
            .block_on(db::ensure_vendas_tables(&pool))
            .expect("vendas");
        runtime
            .block_on(db::ensure_estoque_tables(&pool))
            .expect("estoque");
        runtime
            .block_on(db::ensure_pedidos_tables(&pool))
            .expect("pedidos");
        runtime
            .block_on(db::ensure_tecido_custo_base_column(&pool))
            .expect("custo base tecido");
        runtime
            .block_on(db::ensure_vinculo_image_columns(&pool))
            .expect("imagens vinculos");
        let tecidos = runtime
            .block_on(db::list_tecidos(&pool))
            .unwrap_or_default();
        let cores = runtime.block_on(db::list_cores(&pool)).unwrap_or_default();
        let estampas = runtime
            .block_on(db::list_estampas(&pool))
            .unwrap_or_default();
        let vendas = runtime.block_on(db::list_vendas(&pool)).unwrap_or_default();
        let pedidos = runtime
            .block_on(db::list_pedidos(&pool))
            .unwrap_or_default();
        let app = App::new(
            Some(pool.clone()),
            tecidos,
            cores,
            estampas,
            Vec::new(),
            None,
            3.0,
            vendas,
            pedidos,
            String::from("Shopee nao verificada nos testes"),
            Picker::halfblocks(),
            String::from("Preview: Halfblocks fallback"),
            runtime,
        );
        (app, pool)
    }

    fn cleanup_named_test_data(app: &App, tecidos: &[&str], cores: &[&str], estampas: &[&str]) {
        for tecido in tecidos {
            app.db_runtime
                .block_on(
                    sqlx::query(
                        "DELETE FROM tecido_cores WHERE tecido_id IN (SELECT id FROM tecidos WHERE nome = $1)",
                    )
                    .bind(*tecido)
                    .execute(app.db_pool.as_ref().expect("pool")),
                )
                .expect("limpar tecido_cores por tecido");
            app.db_runtime
                .block_on(
                    sqlx::query(
                        "DELETE FROM tecido_estampas WHERE tecido_id IN (SELECT id FROM tecidos WHERE nome = $1)",
                    )
                    .bind(*tecido)
                    .execute(app.db_pool.as_ref().expect("pool")),
                )
                .expect("limpar tecido_estampas por tecido");
            app.db_runtime
                .block_on(
                    sqlx::query("DELETE FROM tecidos WHERE nome = $1")
                        .bind(*tecido)
                        .execute(app.db_pool.as_ref().expect("pool")),
                )
                .expect("limpar tecido");
        }
        for cor in cores {
            app.db_runtime
                .block_on(
                    sqlx::query(
                        "DELETE FROM tecido_cores WHERE cor_id IN (SELECT id FROM cores WHERE nome = $1)",
                    )
                    .bind(*cor)
                    .execute(app.db_pool.as_ref().expect("pool")),
                )
                .expect("limpar tecido_cores por cor");
            app.db_runtime
                .block_on(
                    sqlx::query("DELETE FROM cores WHERE nome = $1")
                        .bind(*cor)
                        .execute(app.db_pool.as_ref().expect("pool")),
                )
                .expect("limpar cor");
        }
        for estampa in estampas {
            app.db_runtime
                .block_on(
                    sqlx::query(
                        "DELETE FROM tecido_estampas WHERE estampa_id IN (SELECT id FROM estampas WHERE nome = $1)",
                    )
                    .bind(*estampa)
                    .execute(app.db_pool.as_ref().expect("pool")),
                )
                .expect("limpar tecido_estampas por estampa");
            app.db_runtime
                .block_on(
                    sqlx::query("DELETE FROM estampas WHERE nome = $1")
                        .bind(*estampa)
                        .execute(app.db_pool.as_ref().expect("pool")),
                )
                .expect("limpar estampa");
        }
    }

    fn create_tecido_via_agent(app: &mut App, nome: &str) {
        app.submit_dashboard_agent_message(&format!("Quero criar um tecido chamado \"{nome}\""));
        assert_last_contains(app, "composicao");
        assert!(app.handle_pending_agent_draft("100% Algodao"));
        assert_last_contains(app, "largura");
        assert!(app.handle_pending_agent_draft("1,40m rendimento 4"));
        assert_last_contains(app, "Calculo:");
        assert!(app.handle_pending_agent_confirmation("sim"));
        assert_last_contains(app, "Tecido cadastrado");
    }

    fn create_cor_via_agent(app: &mut App, nome: &str, hex: &str) {
        app.submit_dashboard_agent_message(&format!("criar cor \"{nome}\""));
        assert_last_contains(app, "hexadecimal");
        assert!(app.handle_pending_agent_draft(hex));
        assert_last_contains(app, "Cadastrar cor");
        assert!(app.handle_pending_agent_confirmation("sim"));
        assert_last_contains(app, "Cor cadastrada");
    }

    fn create_estampa_via_agent(app: &mut App, nome: &str) {
        app.submit_dashboard_agent_message("criar estampa");
        assert_last_contains(app, "nome da estampa");
        assert!(app.handle_pending_agent_draft(nome));
        assert_last_contains(app, "Cadastrar estampa");
        assert!(app.handle_pending_agent_confirmation("sim"));
        assert_last_contains(app, "Estampa cadastrada");
    }

    fn assert_db_tecido(app: &App, pool: &sqlx::PgPool, nome: &str, composicao: &str) {
        let row = app
            .db_runtime
            .block_on(
                sqlx::query("SELECT composicao FROM tecidos WHERE nome = $1")
                    .bind(nome)
                    .fetch_one(pool),
            )
            .expect("tecido no banco");
        assert_eq!(row.get::<String, _>("composicao"), composicao);
    }

    fn assert_db_cor(app: &App, pool: &sqlx::PgPool, nome: &str, hex: &str) {
        let row = app
            .db_runtime
            .block_on(
                sqlx::query("SELECT codigo_hex FROM cores WHERE nome = $1")
                    .bind(nome)
                    .fetch_one(pool),
            )
            .expect("cor no banco");
        assert_eq!(
            row.get::<Option<String>, _>("codigo_hex").as_deref(),
            Some(hex)
        );
    }

    fn assert_db_estampa(app: &App, pool: &sqlx::PgPool, nome: &str) {
        app.db_runtime
            .block_on(
                sqlx::query("SELECT id FROM estampas WHERE nome = $1")
                    .bind(nome)
                    .fetch_one(pool),
            )
            .expect("estampa no banco");
    }

    fn assert_db_vinculo_cor(app: &App, pool: &sqlx::PgPool, tecido: &str, cor: &str) {
        app.db_runtime
            .block_on(
                sqlx::query(
                    r#"
                    SELECT tc.id
                    FROM tecido_cores tc
                    JOIN tecidos t ON t.id = tc.tecido_id
                    JOIN cores c ON c.id = tc.cor_id
                    WHERE t.nome = $1 AND c.nome = $2
                    "#,
                )
                .bind(tecido)
                .bind(cor)
                .fetch_one(pool),
            )
            .expect("vinculo cor no banco");
    }

    fn assert_last_contains(app: &App, expected: &str) {
        let last = app.chat.messages.last().expect("ultima mensagem");
        assert!(
            last.text.contains(expected),
            "esperava '{expected}' em '{}'",
            last.text
        );
    }
}
