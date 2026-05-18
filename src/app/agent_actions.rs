use super::App;
use crate::{
    agent, db,
    models::{
        AgentAction, CorForm, EstampaForm, Section, SelectValue, TIPO_OPTIONS, TecidoForm,
        VendasScreen, parse_hex_color,
    },
};

impl App {
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
            self.chat
                .messages
                .push(crate::models::ChatMessage::assistant(String::from(
                    "Acao cancelada.",
                )));
            return true;
        }
        false
    }

    pub(super) fn submit_dashboard_agent_message(&mut self, message: &str) -> String {
        if let Some(reply) = self.answer_dashboard_query(message) {
            return reply;
        }
        if let Some(action) = parse_agent_action(message) {
            let description = describe_action(&action);
            self.pending_agent_action = Some(action);
            return format!(
                "{description}\n\nResponda 'sim' para confirmar ou 'nao' para cancelar."
            );
        }

        let skill = self.active_skill();
        let context = self.dashboard_agent_context();
        self.db_runtime
            .block_on(agent::openrouter_reply_with_context(&skill, message, &context))
            .unwrap_or_else(|error| {
                format!(
                    "{}\n\nSkill ativa: {}. Posso consultar dados locais e preparar acoes com confirmacao.",
                    error, skill.name
                )
            })
    }

    fn answer_dashboard_query(&self, message: &str) -> Option<String> {
        let normalized = normalize(message);
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
            } => self.agent_create_tecido(&nome, &composicao, &largura, tipo.as_deref()),
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
                    if let Some(pool) = &self.db_pool {
                        if let Err(error) = self.db_runtime.block_on(db::set_config(
                            pool,
                            "receipt_printer",
                            &printer,
                        )) {
                            return format!("Erro ao salvar impressora: {error}");
                        }
                    }
                    format!("Impressora selecionada: {printer}.")
                } else {
                    format!("Impressora nao encontrada: {printer}.")
                }
            }
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
        if let Some(pool) = &self.db_pool {
            if let Err(error) = self
                .db_runtime
                .block_on(db::insert_cor(pool, nome, &sku, hex))
            {
                return format!("Erro ao cadastrar cor: {error}");
            }
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
        if let Some(pool) = &self.db_pool {
            if let Err(error) = self
                .db_runtime
                .block_on(db::insert_estampa(pool, nome, &sku))
            {
                return format!("Erro ao cadastrar estampa: {error}");
            }
        }
        self.reload_estampas();
        format!("Estampa cadastrada: {nome} ({sku}).")
    }

    fn agent_create_tecido(
        &mut self,
        nome: &str,
        composicao: &str,
        largura: &str,
        tipo: Option<&str>,
    ) -> String {
        let form = TecidoForm {
            nome: nome.to_string(),
            composicao: composicao.to_string(),
            largura: largura.to_string(),
            tipo: tipo
                .map(|value| SelectValue::from_value(value, TIPO_OPTIONS))
                .unwrap_or_default(),
            ..TecidoForm::default()
        };
        if !form.is_valid() {
            return String::from(
                "Dados obrigatorios invalidos para tecido: nome, composicao e largura.",
            );
        }
        let sku = form.sku(&self.tecidos, None);
        if let Some(pool) = &self.db_pool {
            if let Err(error) = self
                .db_runtime
                .block_on(db::insert_tecido(pool, &form, &sku))
            {
                return format!("Erro ao cadastrar tecido: {error}");
            }
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

    fn dashboard_agent_context(&self) -> String {
        format!(
            "Dashboard master. Tecidos: {}. Cores: {}. Estampas: {}. Vendas no periodo carregado: {}. Impressora: {}. Pode preparar acoes com confirmacao: cadastrar tecido/cor/estampa, criar vinculos, abrir venda, filtrar historico, selecionar impressora.",
            self.tecidos.len(),
            self.cores.len(),
            self.estampas.len(),
            self.vendas_historico.len(),
            self.selected_printer.as_deref().unwrap_or("nenhuma")
        )
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
        let printer = value_after(message, "impressora")
            .or_else(|| value_after(message, "printer"))
            .unwrap_or_else(|| message.to_string());
        return Some(AgentAction::SelectPrinter { printer });
    }
    None
}

fn describe_action(action: &AgentAction) -> String {
    match action {
        AgentAction::CreateTecido {
            nome,
            composicao,
            largura,
            tipo,
        } => {
            format!(
                "Cadastrar tecido '{nome}', composicao '{composicao}', largura '{largura}', tipo '{}'.",
                tipo.as_deref().unwrap_or("Selecione")
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
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
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
    let end = tail.find(|ch| ch == ',' || ch == ';').unwrap_or(tail.len());
    Some(tail[..end].trim().to_string()).filter(|item| !item.is_empty())
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
