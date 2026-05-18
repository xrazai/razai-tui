use std::{time::Duration, time::Instant};

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
    db::{CorRecord, EstampaRecord, TecidoRecord, VendaHistoricoRecord, VinculoRecord},
    models::*,
    screens,
};

mod configuracoes;
mod dados;
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
    pub vendas_historico: Vec<VendaHistoricoRecord>,
    pub venda_historico_option: usize,
    pub editing_venda_id: Option<i64>,
    pub finalizar_venda_dialog: bool,
    pub finalizar_venda_option: FinalizarVendaOption,
    pub pending_delete_venda: bool,
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
        db_runtime: Runtime,
    ) -> Self {
        let printers = configuracoes::list_installed_printers();
        let selected_printer = selected_printer.filter(|printer| printers.contains(printer));
        let printer_option = selected_printer
            .as_ref()
            .and_then(|selected| printers.iter().position(|printer| printer == selected))
            .unwrap_or(0);

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
            vendas_historico,
            venda_historico_option: 0,
            editing_venda_id: None,
            finalizar_venda_dialog: false,
            finalizar_venda_option: FinalizarVendaOption::default(),
            pending_delete_venda: false,
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

    fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.running = false;
            return;
        }

        if key.code == KeyCode::F(2) {
            self.focus = self.focus.toggle();
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
            KeyCode::Char('6') => self.section = Section::Configuracoes,
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
            KeyCode::Left | KeyCode::BackTab => self.section = self.section.previous(),
            KeyCode::Right | KeyCode::Tab => self.section = self.section.next(),
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

    fn submit_chat(&mut self) {
        let message = self.chat.input.trim().to_string();
        if message.is_empty() {
            return;
        }

        self.chat.messages.push(ChatMessage::user(message.clone()));
        self.chat.input.clear();

        let skill = self.active_skill();
        let reply = self
            .db_runtime
            .block_on(agent::openrouter_reply(&skill, &message, &self.tecido_form))
            .unwrap_or_else(|error| {
                format!(
                    "{}\n\n{}",
                    error,
                    agent::local_reply(&skill, &message, &self.tecido_form)
                )
            });
        self.chat.messages.push(ChatMessage::assistant(reply));
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
            .constraints([Constraint::Min(50), Constraint::Length(38)])
            .split(outer[2]);

        match self.section {
            Section::Vendas => screens::vendas::render(frame, body[0], self),
            Section::Dados => screens::dados::render(frame, body[0], self),
            Section::Configuracoes => screens::configuracoes::render(frame, body[0], self),
            section => screens::chrome::render_content(frame, body[0], section),
        }

        screens::chrome::render_chat(frame, body[1], &self.chat, self.focus, &self.active_skill());
        screens::chrome::render_footer(frame, outer[3], &self.db_status, self.focus);
    }

    fn active_skill(&self) -> agent::SkillContext {
        agent::active_skill(
            self.section,
            self.dados_screen,
            self.dados_option,
            self.vendas_screen,
        )
    }
}
