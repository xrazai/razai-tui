use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
    time::Instant,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
};
use sqlx::PgPool;
use tokio::runtime::Runtime;

use crate::{
    agent,
    db::{
        self, CorRecord, EstampaRecord, PedidoRecord, TecidoRecord, VendaHistoricoRecord,
        VinculoRecord,
    },
    models::*,
    screens,
    shopee,
    ui::SIDE_PANEL_WIDTH,
};

mod agent_actions;
mod configuracoes;
mod dados;
mod pedidos;
mod vendas;

use std::io;

pub struct App {
    pub section: Section,
    pub dados_screen: DadosScreen,
    pub dados_option: DadosOption,
    pub tecido_option: usize,
    pub cor_option: usize,
    pub vinculo_menu_option: usize,
    pub vinculo_tecido_option: usize,
    pub vinculo_criar_option: usize,
    pub vinculo_lista_option: usize,
    pub tecidos: Vec<TecidoRecord>,
    pub cores: Vec<CorRecord>,
    pub estampas: Vec<EstampaRecord>,
    pub vinculos: Vec<VinculoRecord>,
    pub selected_vinculo_cores: Vec<i64>,
    pub tecido_form: TecidoForm,
    pub tecido_select_dropdown: Option<TecidoField>,
    pub cor_form: CorForm,
    pub estampa_form: EstampaForm,
    pub editing_tecido_id: Option<i64>,
    pub editing_cor_id: Option<i64>,
    pub editing_estampa_id: Option<i64>,
    pub pending_delete: bool,
    pub db_pool: Option<PgPool>,
    pub db_runtime: Runtime,
    pub db_status: String,
    pub focus: Focus,
    pub chat: ChatState,
    pub chat_reply_rx: Option<Receiver<String>>,
    pub pending_agent_draft: Option<AgentDraft>,
    pub pending_agent_action: Option<AgentAction>,
    pub vendas_screen: VendasScreen,
    pub venda_menu_option: usize,
    pub venda_tecido_option: usize,
    pub venda_vinculo_option: usize,
    pub venda_field: VendaField,
    pub venda_dropdown: Option<VendaField>,
    pub venda_preco: String,
    pub venda_quantidade: String,
    pub venda_vinculos: Vec<VinculoRecord>,
    pub venda_itens: Vec<VendaItem>,
    pub venda_resumo_focus: bool,
    pub venda_item_option: usize,
    pub editing_venda_item: Option<usize>,
    pub editing_venda_item_descricao: Option<String>,
    pub vendas_historico: Vec<VendaHistoricoRecord>,
    pub venda_historico_option: usize,
    pub venda_historico_field: usize,
    pub venda_historico_inicio: String,
    pub venda_historico_fim: String,
    pub editing_venda_id: Option<i64>,
    pub finalizar_venda_dialog: bool,
    pub finalizar_venda_option: FinalizarVendaOption,
    pub pending_delete_venda: bool,
    pub pending_delete_venda_item: bool,
    pub pedidos_screen: PedidosScreen,
    pub pedido_menu_option: usize,
    pub pedido_tecido_option: usize,
    pub pedido_vinculo_option: usize,
    pub pedido_field: VendaField,
    pub pedido_dropdown: Option<VendaField>,
    pub pedido_preco: String,
    pub pedido_quantidade: String,
    pub pedido_vinculos: Vec<VinculoRecord>,
    pub pedido_itens: Vec<VendaItem>,
    pub pedido_resumo_focus: bool,
    pub pedido_item_option: usize,
    pub pedidos_historico: Vec<PedidoRecord>,
    pub pedido_historico_option: usize,
    pub editing_pedido_id: Option<i64>,
    pub finalizar_pedido_dialog: bool,
    pub finalizar_pedido_option: FinalizarVendaOption,
    pub pending_approve_pedido: bool,
    pub shopee_menu_option: usize,
    pub shopee_status: String,
    pub printers: Vec<String>,
    pub printer_option: usize,
    pub selected_printer: Option<String>,
    pub running: bool,
}

impl App {
    pub fn new(
        db_pool: Option<PgPool>,
        tecidos: Vec<TecidoRecord>,
        cores: Vec<CorRecord>,
        estampas: Vec<EstampaRecord>,
        selected_printer: Option<String>,
        vendas_historico: Vec<VendaHistoricoRecord>,
        pedidos_historico: Vec<PedidoRecord>,
        shopee_status: String,
        db_runtime: Runtime,
    ) -> Self {
        let printers = configuracoes::list_installed_printers();
        let selected_printer = selected_printer.filter(|printer| printers.contains(printer));
        let printer_option = selected_printer
            .as_ref()
            .and_then(|selected| printers.iter().position(|printer| printer == selected))
            .unwrap_or(0);
        let today = db::format_sales_date(db::today_sales_date());

        Self {
            section: Section::default(),
            dados_screen: DadosScreen::default(),
            dados_option: DadosOption::default(),
            tecido_option: 0,
            cor_option: 0,
            vinculo_menu_option: 0,
            vinculo_tecido_option: 0,
            vinculo_criar_option: 0,
            vinculo_lista_option: 0,
            tecidos,
            cores,
            estampas,
            vinculos: Vec::new(),
            selected_vinculo_cores: Vec::new(),
            tecido_form: TecidoForm::default(),
            tecido_select_dropdown: None,
            cor_form: CorForm::default(),
            estampa_form: EstampaForm::default(),
            editing_tecido_id: None,
            editing_cor_id: None,
            editing_estampa_id: None,
            pending_delete: false,
            db_status: if db_pool.is_some() {
                String::from("Banco local conectado")
            } else {
                String::from("Banco local indisponivel")
            },
            db_pool,
            db_runtime,
            focus: Focus::System,
            chat: ChatState::default(),
            chat_reply_rx: None,
            pending_agent_draft: None,
            pending_agent_action: None,
            vendas_screen: VendasScreen::default(),
            venda_menu_option: 0,
            venda_tecido_option: 0,
            venda_vinculo_option: 0,
            venda_field: VendaField::default(),
            venda_dropdown: None,
            venda_preco: String::new(),
            venda_quantidade: String::new(),
            venda_vinculos: Vec::new(),
            venda_itens: Vec::new(),
            venda_resumo_focus: false,
            venda_item_option: 0,
            editing_venda_item: None,
            editing_venda_item_descricao: None,
            vendas_historico,
            venda_historico_option: 0,
            venda_historico_field: 2,
            venda_historico_inicio: today.clone(),
            venda_historico_fim: today,
            editing_venda_id: None,
            finalizar_venda_dialog: false,
            finalizar_venda_option: FinalizarVendaOption::default(),
            pending_delete_venda: false,
            pending_delete_venda_item: false,
            pedidos_screen: PedidosScreen::default(),
            pedido_menu_option: 0,
            pedido_tecido_option: 0,
            pedido_vinculo_option: 0,
            pedido_field: VendaField::default(),
            pedido_dropdown: None,
            pedido_preco: String::new(),
            pedido_quantidade: String::new(),
            pedido_vinculos: Vec::new(),
            pedido_itens: Vec::new(),
            pedido_resumo_focus: false,
            pedido_item_option: 0,
            pedidos_historico,
            pedido_historico_option: 0,
            editing_pedido_id: None,
            finalizar_pedido_dialog: false,
            finalizar_pedido_option: FinalizarVendaOption::default(),
            pending_approve_pedido: false,
            shopee_menu_option: 0,
            shopee_status,
            printers,
            printer_option,
            selected_printer,
            running: false,
        }
    }

    pub fn run(mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        self.running = true;
        let mut last_tick = Instant::now();

        while self.running {
            self.drain_chat_reply();
            terminal.draw(|frame| self.render(frame))?;

            let timeout = Duration::from_millis(250)
                .saturating_sub(last_tick.elapsed())
                .max(Duration::from_millis(10));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key);
                    }
                }
            }

            if last_tick.elapsed() >= Duration::from_millis(250) {
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    fn drain_chat_reply(&mut self) {
        let reply = self
            .chat_reply_rx
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(reply) = reply {
            self.chat.waiting = false;
            self.chat_reply_rx = None;
            self.chat.messages.push(ChatMessage::assistant(reply));
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.running = false;
            return;
        }

        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
            self.handle_focus_tab(key.code);
            return;
        }

        if self.focus == Focus::Chat {
            self.handle_chat_key(key.code);
            return;
        }

        if self.section == Section::Dados && self.dados_screen == DadosScreen::CadastrarTecido {
            self.handle_tecido_form_key(key.code);
            return;
        }
        if self.section == Section::Dados && self.dados_screen == DadosScreen::CadastrarCor {
            self.handle_cor_form_key(key.code);
            return;
        }
        if self.section == Section::Dados && self.dados_screen == DadosScreen::CadastrarEstampa {
            self.handle_estampa_form_key(key.code);
            return;
        }
        if self.section == Section::Vendas {
            self.handle_vendas_key(key.code);
            return;
        }
        if self.section == Section::Pedidos {
            self.handle_pedidos_key(key.code);
            return;
        }
        if self.section == Section::Shopee {
            self.handle_shopee_key(key.code);
            return;
        }
        if self.section == Section::Configuracoes {
            self.handle_configuracoes_key(key.code);
            return;
        }

        match key.code {
            KeyCode::Esc if self.section == Section::Dados => self.voltar_dados(),
            KeyCode::Esc => self.running = false,
            KeyCode::Char('1') => self.section = Section::Dashboard,
            KeyCode::Char('2') => self.section = Section::Vendas,
            KeyCode::Char('3') => self.section = Section::Pedidos,
            KeyCode::Char('4') => self.section = Section::Dados,
            KeyCode::Char('5') => self.section = Section::Estoque,
            KeyCode::Char('6') => self.section = Section::Shopee,
            KeyCode::Char('7') => self.section = Section::Configuracoes,
            KeyCode::Backspace if self.section == Section::Dados => {}
            KeyCode::Up if self.section == Section::Dados => match self.dados_screen {
                DadosScreen::Menu => self.dados_option = self.dados_option.previous(),
                DadosScreen::Tecidos => self.previous_tecido(),
                DadosScreen::Cores => self.previous_cor(),
                DadosScreen::Estampas => self.previous_cor(),
                DadosScreen::VinculosMenu => self.previous_vinculo_menu(),
                DadosScreen::VinculosSelecionarTecidoCriar
                | DadosScreen::VinculosSelecionarTecidoVer => self.previous_vinculo_tecido(),
                DadosScreen::VinculosSelecionarCores => self.previous_vinculo_criar_option(),
                DadosScreen::VinculosLista => self.previous_vinculo_lista(),
                DadosScreen::CadastrarTecido
                | DadosScreen::CadastrarCor
                | DadosScreen::CadastrarEstampa => {}
            },
            KeyCode::Down if self.section == Section::Dados => match self.dados_screen {
                DadosScreen::Menu => self.dados_option = self.dados_option.next(),
                DadosScreen::Tecidos => self.next_tecido(),
                DadosScreen::Cores => self.next_cor(),
                DadosScreen::Estampas => self.next_cor(),
                DadosScreen::VinculosMenu => self.next_vinculo_menu(),
                DadosScreen::VinculosSelecionarTecidoCriar
                | DadosScreen::VinculosSelecionarTecidoVer => self.next_vinculo_tecido(),
                DadosScreen::VinculosSelecionarCores => self.next_vinculo_criar_option(),
                DadosScreen::VinculosLista => self.next_vinculo_lista(),
                DadosScreen::CadastrarTecido
                | DadosScreen::CadastrarCor
                | DadosScreen::CadastrarEstampa => {}
            },
            KeyCode::Enter if self.section == Section::Dados => {
                if self.dados_screen == DadosScreen::Menu {
                    match self.dados_option {
                        DadosOption::Tecido => {
                            self.dados_screen = DadosScreen::Tecidos;
                            self.tecido_option = 0;
                        }
                        DadosOption::Cores => {
                            self.dados_screen = DadosScreen::Cores;
                            self.cor_option = 0;
                        }
                        DadosOption::Estampas => {
                            self.dados_screen = DadosScreen::Estampas;
                            self.cor_option = 0;
                        }
                        DadosOption::Vinculos => {
                            self.dados_screen = DadosScreen::VinculosMenu;
                            self.vinculo_menu_option = 0;
                        }
                    }
                } else if self.dados_screen == DadosScreen::Tecidos {
                    if self.tecido_option == 0 {
                        self.open_new_tecido();
                    } else {
                        self.open_edit_tecido(self.tecido_option - 1);
                    }
                } else if self.dados_screen == DadosScreen::Cores {
                    if self.cor_option == 0 {
                        self.open_new_cor();
                    } else {
                        self.open_edit_cor(self.cor_option - 1);
                    }
                } else if self.dados_screen == DadosScreen::Estampas {
                    if self.cor_option == 0 {
                        self.open_new_estampa();
                    } else {
                        self.open_edit_estampa(self.cor_option - 1);
                    }
                } else if self.dados_screen == DadosScreen::VinculosMenu {
                    if self.vinculo_menu_option == 0 {
                        self.dados_screen = DadosScreen::VinculosSelecionarTecidoCriar;
                    } else {
                        self.dados_screen = DadosScreen::VinculosSelecionarTecidoVer;
                    }
                    self.vinculo_tecido_option = 0;
                } else if self.dados_screen == DadosScreen::VinculosSelecionarTecidoCriar {
                    self.open_vinculo_cores();
                } else if self.dados_screen == DadosScreen::VinculosSelecionarTecidoVer {
                    self.open_vinculo_lista();
                } else if self.dados_screen == DadosScreen::VinculosSelecionarCores {
                    self.handle_vinculo_criar_enter();
                }
            }
            KeyCode::Char(' ') if self.dados_screen == DadosScreen::VinculosSelecionarCores => {
                self.toggle_vinculo_cor();
            }
            KeyCode::Left => self.section = self.section.previous(),
            KeyCode::Right => self.section = self.section.next(),
            _ => {}
        }
    }

    fn handle_chat_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.focus = Focus::System,
            KeyCode::Backspace => {
                self.chat.input.pop();
            }
            KeyCode::Enter => self.submit_chat(),
            KeyCode::Char(character) if !character.is_control() => self.chat.input.push(character),
            _ => {}
        }
    }

    fn handle_focus_tab(&mut self, key: KeyCode) {
        if self.focus == Focus::Chat {
            self.focus = match key {
                KeyCode::BackTab => self.focus.previous(),
                _ => self.focus.next(),
            };
            return;
        }

        if self.section == Section::Vendas && self.vendas_screen == VendasScreen::Lancamento {
            if key == KeyCode::BackTab {
                if self.venda_resumo_focus {
                    self.venda_resumo_focus = false;
                } else {
                    self.focus = Focus::Chat;
                }
            } else if self.venda_resumo_focus {
                self.focus = Focus::Chat;
                self.venda_resumo_focus = false;
            } else if self.venda_itens.is_empty() {
                self.focus = Focus::Chat;
            } else {
                self.venda_resumo_focus = true;
            }
            return;
        }

        if self.section == Section::Pedidos && self.pedidos_screen == PedidosScreen::Lancamento {
            if key == KeyCode::BackTab {
                if self.pedido_resumo_focus {
                    self.pedido_resumo_focus = false;
                } else {
                    self.focus = Focus::Chat;
                }
            } else if self.pedido_resumo_focus {
                self.focus = Focus::Chat;
                self.pedido_resumo_focus = false;
            } else if self.pedido_itens.is_empty() {
                self.focus = Focus::Chat;
            } else {
                self.pedido_resumo_focus = true;
            }
            return;
        }

        self.focus = match key {
            KeyCode::BackTab => self.focus.previous(),
            _ => self.focus.next(),
        };
    }

    fn handle_shopee_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.running = false,
            KeyCode::Up | KeyCode::Down => {
                self.shopee_menu_option = (self.shopee_menu_option + 1) % 2;
            }
            KeyCode::Enter => {
                let status = match self.shopee_menu_option {
                    0 => self
                        .db_runtime
                        .block_on(shopee::create_listing_status(self.db_pool.as_ref())),
                    _ => self
                        .db_runtime
                        .block_on(shopee::online_stock_summary(self.db_pool.as_ref())),
                };
                self.shopee_status = status.clone();
                self.db_status = status;
            }
            KeyCode::Char('1') => self.shopee_menu_option = 0,
            KeyCode::Char('2') => self.shopee_menu_option = 1,
            _ => {}
        }
    }

    fn submit_chat(&mut self) {
        if self.chat.waiting {
            self.chat.messages.push(ChatMessage::assistant(String::from(
                "Ainda estou processando a mensagem anterior.",
            )));
            return;
        }

        let message = self.chat.input.trim().to_string();
        if message.is_empty() {
            return;
        }

        self.chat.messages.push(ChatMessage::user(message.clone()));
        self.chat.input.clear();

        if self.handle_pending_agent_confirmation(&message) {
            return;
        }

        if self.handle_pending_agent_draft(&message) {
            return;
        }

        if self.section == Section::Dashboard {
            self.submit_dashboard_agent_message(&message);
            return;
        }

        if self.section == Section::Shopee {
            if wants_shopee_callback(&message) {
                let reply = match shopee::start_callback_listener(self.db_pool.clone()) {
                    Ok(status) => {
                        self.shopee_status = status.clone();
                        self.db_status = String::from("Shopee aguardando callback local");
                        status
                    }
                    Err(error) => format!("Shopee: falha ao iniciar callback local: {error}"),
                };
                self.chat.messages.push(ChatMessage::assistant(reply));
                return;
            }
            if let Some(code) = parse_shopee_code(&message) {
                let reply = match self
                    .db_runtime
                    .block_on(shopee::exchange_code(self.db_pool.as_ref(), code))
                {
                    Ok(()) => {
                        self.shopee_status = String::from("Shopee conectada; tokens salvos");
                        self.db_status = self.shopee_status.clone();
                        self.shopee_status.clone()
                    }
                    Err(error) => format!("Shopee: falha ao trocar code por tokens: {error}"),
                };
                self.chat.messages.push(ChatMessage::assistant(reply));
                return;
            }
        }

        if self.submit_context_agent_message(&message) {
            return;
        }

        let context_info = self.active_context();
        let context = self.agent_context();
        let fallback = agent::local_reply(&context_info, &message, &self.tecido_form);
        self.spawn_agent_reply(context_info, message, context, fallback);
    }

    pub(super) fn spawn_agent_reply(
        &mut self,
        context_info: agent::AgentContext,
        message: String,
        context: String,
        fallback: String,
    ) {
        let (sender, receiver) = mpsc::channel();
        self.chat.waiting = true;
        self.chat_reply_rx = Some(receiver);
        thread::spawn(move || {
            let reply = match Runtime::new() {
                Ok(runtime) => runtime
                    .block_on(agent::openrouter_reply_with_context(
                        &context_info,
                        &message,
                        &context,
                    ))
                    .unwrap_or_else(|error| format!("{error}\n\n{fallback}")),
                Err(error) => format!("Erro ao iniciar runtime do agente: {error}\n\n{fallback}"),
            };
            let _ = sender.send(reply);
        });
    }

    pub(super) fn agent_context(&self) -> String {
        let tecidos = self
            .tecidos
            .iter()
            .take(12)
            .map(|tecido| {
                format!(
                    "{} [{}] tipo={} largura={:.2}m",
                    tecido.nome, tecido.sku, tecido.tipo, tecido.largura_m
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let cores = self
            .cores
            .iter()
            .take(12)
            .map(|cor| {
                format!(
                    "{} [{}] {}",
                    cor.nome,
                    cor.sku.as_deref().unwrap_or("sem SKU"),
                    cor.codigo_hex.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let estampas = self
            .estampas
            .iter()
            .take(12)
            .map(|estampa| {
                format!(
                    "{} [{}]",
                    estampa.nome,
                    estampa.sku.as_deref().unwrap_or("sem SKU")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let vendas = self
            .vendas_historico
            .iter()
            .take(8)
            .map(|venda| {
                format!(
                    "#{} {} {} itens R${}",
                    venda.id,
                    venda.created_at,
                    venda.itens,
                    format_money(venda.total)
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let pedidos = self
            .pedidos_historico
            .iter()
            .take(8)
            .map(|pedido| {
                format!(
                    "#{} {} {} {} itens R${}",
                    pedido.id,
                    pedido.created_at,
                    pedido.status,
                    pedido.itens,
                    format_money(pedido.total)
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let venda_total = self.venda_itens.iter().map(VendaItem::total).sum::<f64>();
        let pedido_total = self.pedido_itens.iter().map(VendaItem::total).sum::<f64>();

        format!(
            "Projeto Razai TUI: sistema terminal para loja de tecidos com Dashboard, Vendas, Pedidos, Dados, Estoque e Configuracoes. Tela atual: {}. Status: {}. Dados carregados: {} tecidos, {} cores, {} estampas, {} vendas no periodo {}..{}, {} pedidos. Tecidos: {}. Cores: {}. Estampas: {}. Vendas recentes: {}. Pedidos recentes: {}. Venda em andamento: {} itens, total R${}, preco='{}', quantidade='{}'. Pedido em andamento: {} itens, total R${}, preco='{}', quantidade='{}'. Impressora: {}. Formulario tecido: nome='{}', composicao='{}', largura='{}', tipo='{}'. Regras: gravacoes exigem confirmacao; vendas viram historico; pedidos geram PDF em pdf_pedidos e podem ser aprovados como venda.",
            self.section.title(),
            self.db_status,
            self.tecidos.len(),
            self.cores.len(),
            self.estampas.len(),
            self.vendas_historico.len(),
            self.venda_historico_inicio,
            self.venda_historico_fim,
            self.pedidos_historico.len(),
            empty_label(&tecidos),
            empty_label(&cores),
            empty_label(&estampas),
            empty_label(&vendas),
            empty_label(&pedidos),
            self.venda_itens.len(),
            format_money(venda_total),
            self.venda_preco,
            self.venda_quantidade,
            self.pedido_itens.len(),
            format_money(pedido_total),
            self.pedido_preco,
            self.pedido_quantidade,
            self.selected_printer.as_deref().unwrap_or("nenhuma"),
            self.tecido_form.nome,
            self.tecido_form.composicao,
            self.tecido_form.largura,
            self.tecido_form.tipo.value(TIPO_OPTIONS)
        )
    }

    fn render(&self, frame: &mut Frame) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(frame.area());

        screens::chrome::render_header(frame, outer[0]);
        screens::chrome::render_tabs(frame, outer[1], self.section);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(50), Constraint::Length(SIDE_PANEL_WIDTH)])
            .split(outer[2]);

        match self.section {
            Section::Vendas => screens::vendas::render(frame, body[0], self),
            Section::Pedidos => screens::pedidos::render(frame, body[0], self),
            Section::Dados => screens::dados::render(frame, body[0], self),
            Section::Shopee => screens::shopee::render(
                frame,
                body[0],
                self.shopee_menu_option,
                &self.shopee_status,
            ),
            Section::Configuracoes => screens::configuracoes::render(frame, body[0], self),
            section => screens::chrome::render_content(frame, body[0], section),
        }

        screens::chrome::render_chat(
            frame,
            body[1],
            &self.chat,
            self.focus,
            &self.active_context(),
        );
        screens::chrome::render_footer(frame, outer[3], &self.db_status, self.focus);
    }

    fn active_context(&self) -> agent::AgentContext {
        agent::active_context(
            self.section,
            self.dados_screen,
            self.dados_option,
            self.vendas_screen,
        )
    }
}

fn format_money(value: f64) -> String {
    format!("{value:.2}").replace('.', ",")
}

fn empty_label(value: &str) -> &str {
    if value.trim().is_empty() {
        "nenhum"
    } else {
        value
    }
}

fn parse_shopee_code(message: &str) -> Option<&str> {
    let message = message.trim();
    for prefix in ["code=", "code:", "code "] {
        if let Some(code) = message.strip_prefix(prefix) {
            return non_empty_code(code);
        }
    }
    if let Some((_, query)) = message.split_once('?') {
        for part in query.split('&') {
            if let Some(code) = part.strip_prefix("code=") {
                return non_empty_code(code);
            }
        }
    }
    None
}

fn non_empty_code(code: &str) -> Option<&str> {
    let code = code.trim();
    (!code.is_empty()).then_some(code)
}

fn wants_shopee_callback(message: &str) -> bool {
    let message = message.trim().to_ascii_lowercase();
    matches!(
        message.as_str(),
        "conectar" | "conectar shopee" | "callback" | "iniciar callback" | "ngrok"
    )
}
