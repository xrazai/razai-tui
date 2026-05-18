use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
};
use sqlx::PgPool;
use tokio::runtime::Runtime;

use crate::{
    agent, db,
    db::{CorRecord, TecidoRecord, VinculoRecord},
    models::*,
    ui::{centered_rect, color_swatch, selected_style},
};

use std::io;

pub struct App {
    section: Section,
    dados_screen: DadosScreen,
    dados_option: DadosOption,
    tecido_option: usize,
    cor_option: usize,
    vinculo_menu_option: usize,
    vinculo_tecido_option: usize,
    vinculo_criar_option: usize,
    vinculo_lista_option: usize,
    tecidos: Vec<TecidoRecord>,
    cores: Vec<CorRecord>,
    vinculos: Vec<VinculoRecord>,
    selected_vinculo_cores: Vec<i64>,
    tecido_form: TecidoForm,
    cor_form: CorForm,
    editing_tecido_id: Option<i64>,
    editing_cor_id: Option<i64>,
    pending_delete: bool,
    db_pool: Option<PgPool>,
    db_runtime: Runtime,
    db_status: String,
    focus: Focus,
    chat: ChatState,
    vendas_screen: VendasScreen,
    venda_menu_option: usize,
    venda_tecido_option: usize,
    venda_tipo_option: usize,
    venda_vinculo_option: usize,
    venda_field: VendaField,
    venda_preco: String,
    venda_quantidade: String,
    venda_vinculos: Vec<VinculoRecord>,
    venda_itens: Vec<VendaItem>,
    running: bool,
}

impl App {
    pub fn new(
        db_pool: Option<PgPool>,
        tecidos: Vec<TecidoRecord>,
        cores: Vec<CorRecord>,
        db_runtime: Runtime,
    ) -> Self {
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
            vinculos: Vec::new(),
            selected_vinculo_cores: Vec::new(),
            tecido_form: TecidoForm::default(),
            cor_form: CorForm::default(),
            editing_tecido_id: None,
            editing_cor_id: None,
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
            venda_tipo_option: 0,
            venda_vinculo_option: 0,
            venda_field: VendaField::default(),
            venda_preco: String::new(),
            venda_quantidade: String::new(),
            venda_vinculos: Vec::new(),
            venda_itens: Vec::new(),
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
        if self.section == Section::Vendas {
            self.handle_vendas_key(key.code);
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
            KeyCode::Backspace if self.section == Section::Dados => {}
            KeyCode::Up if self.section == Section::Dados => match self.dados_screen {
                DadosScreen::Menu => self.dados_option = self.dados_option.previous(),
                DadosScreen::Tecidos => self.previous_tecido(),
                DadosScreen::Cores => self.previous_cor(),
                DadosScreen::VinculosMenu => self.previous_vinculo_menu(),
                DadosScreen::VinculosSelecionarTecidoCriar
                | DadosScreen::VinculosSelecionarTecidoVer => self.previous_vinculo_tecido(),
                DadosScreen::VinculosSelecionarCores => self.previous_vinculo_criar_option(),
                DadosScreen::VinculosLista => self.previous_vinculo_lista(),
                DadosScreen::CadastrarTecido | DadosScreen::CadastrarCor => {}
            },
            KeyCode::Down if self.section == Section::Dados => match self.dados_screen {
                DadosScreen::Menu => self.dados_option = self.dados_option.next(),
                DadosScreen::Tecidos => self.next_tecido(),
                DadosScreen::Cores => self.next_cor(),
                DadosScreen::VinculosMenu => self.next_vinculo_menu(),
                DadosScreen::VinculosSelecionarTecidoCriar
                | DadosScreen::VinculosSelecionarTecidoVer => self.next_vinculo_tecido(),
                DadosScreen::VinculosSelecionarCores => self.next_vinculo_criar_option(),
                DadosScreen::VinculosLista => self.next_vinculo_lista(),
                DadosScreen::CadastrarTecido | DadosScreen::CadastrarCor => {}
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

    fn handle_tecido_form_key(&mut self, key: KeyCode) {
        if self.pending_delete {
            match key {
                KeyCode::Char('s') | KeyCode::Char('S') => self.excluir_tecido_confirmado(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.pending_delete = false
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Esc => self.voltar_dados(),
            KeyCode::Backspace => self.tecido_form.backspace(),
            KeyCode::Up => self.tecido_form.previous_field(),
            KeyCode::Down => self.tecido_form.next_field(),
            KeyCode::Left => self.tecido_form.previous_select_option(),
            KeyCode::Right => self.tecido_form.next_select_option(),
            KeyCode::Enter if self.tecido_form.selected_field == TecidoField::Salvar => {
                self.cadastrar_tecido();
            }
            KeyCode::Enter if self.tecido_form.selected_field == TecidoField::Excluir => {
                self.pending_delete = true;
            }
            KeyCode::Enter if self.tecido_form.selected_field == TecidoField::Voltar => {
                self.voltar_dados();
            }
            KeyCode::Enter => self.tecido_form.next_field(),
            KeyCode::Char(character) => self.tecido_form.push(character),
            KeyCode::BackTab => self.section = self.section.previous(),
            KeyCode::Tab => self.section = self.section.next(),
            _ => {}
        }
    }

    fn handle_cor_form_key(&mut self, key: KeyCode) {
        if self.pending_delete {
            match key {
                KeyCode::Char('s') | KeyCode::Char('S') => self.excluir_cor_confirmada(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.pending_delete = false
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Esc => self.voltar_dados(),
            KeyCode::Backspace => self.cor_form.backspace(),
            KeyCode::Up => self.cor_form.previous_field(),
            KeyCode::Down | KeyCode::Enter => {
                if self.cor_form.selected_field == CorField::Confirmar {
                    self.confirmar_cor();
                } else if self.cor_form.selected_field == CorField::Voltar {
                    self.voltar_dados();
                } else if self.cor_form.selected_field == CorField::Excluir {
                    self.pending_delete = true;
                } else {
                    self.cor_form.next_field();
                }
            }
            KeyCode::Char(character) => self.cor_form.push(character),
            KeyCode::Tab => self.section = self.section.next(),
            KeyCode::BackTab => self.section = self.section.previous(),
            _ => {}
        }
    }

    fn handle_vendas_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.voltar_vendas(),
            KeyCode::Backspace => self.backspace_venda_field(),
            KeyCode::Up => self.previous_venda_option(),
            KeyCode::Down => self.next_venda_option(),
            KeyCode::Enter => self.enter_vendas(),
            KeyCode::Char(character)
                if !character.is_control() && self.vendas_screen == VendasScreen::Lancamento =>
            {
                self.push_venda_field(character);
            }
            _ => {}
        }
    }

    fn next_tecido(&mut self) {
        self.tecido_option = (self.tecido_option + 1) % self.tecidos_menu_len();
    }

    fn previous_tecido(&mut self) {
        self.tecido_option =
            (self.tecido_option + self.tecidos_menu_len() - 1) % self.tecidos_menu_len();
    }

    fn tecidos_menu_len(&self) -> usize {
        self.tecidos.len() + 1
    }

    fn next_cor(&mut self) {
        self.cor_option = (self.cor_option + 1) % (self.cores.len() + 1);
    }

    fn previous_cor(&mut self) {
        let len = self.cores.len() + 1;
        self.cor_option = (self.cor_option + len - 1) % len;
    }

    fn next_vinculo_menu(&mut self) {
        self.vinculo_menu_option = (self.vinculo_menu_option + 1) % 2;
    }

    fn previous_vinculo_menu(&mut self) {
        self.vinculo_menu_option = (self.vinculo_menu_option + 1) % 2;
    }

    fn next_vinculo_tecido(&mut self) {
        if !self.tecidos.is_empty() {
            self.vinculo_tecido_option = (self.vinculo_tecido_option + 1) % self.tecidos.len();
        }
    }

    fn previous_vinculo_tecido(&mut self) {
        if !self.tecidos.is_empty() {
            self.vinculo_tecido_option =
                (self.vinculo_tecido_option + self.tecidos.len() - 1) % self.tecidos.len();
        }
    }

    fn vinculo_criar_len(&self) -> usize {
        self.cores.len() + 2
    }

    fn next_vinculo_criar_option(&mut self) {
        self.vinculo_criar_option = (self.vinculo_criar_option + 1) % self.vinculo_criar_len();
    }

    fn previous_vinculo_criar_option(&mut self) {
        self.vinculo_criar_option =
            (self.vinculo_criar_option + self.vinculo_criar_len() - 1) % self.vinculo_criar_len();
    }

    fn next_vinculo_lista(&mut self) {
        if !self.vinculos.is_empty() {
            self.vinculo_lista_option = (self.vinculo_lista_option + 1) % self.vinculos.len();
        }
    }

    fn previous_vinculo_lista(&mut self) {
        if !self.vinculos.is_empty() {
            self.vinculo_lista_option =
                (self.vinculo_lista_option + self.vinculos.len() - 1) % self.vinculos.len();
        }
    }

    fn voltar_dados(&mut self) {
        match self.dados_screen {
            DadosScreen::CadastrarTecido => {
                self.tecido_form = TecidoForm::default();
                self.editing_tecido_id = None;
                self.pending_delete = false;
                self.dados_screen = DadosScreen::Tecidos;
            }
            DadosScreen::CadastrarCor => {
                self.cor_form = CorForm::default();
                self.editing_cor_id = None;
                self.pending_delete = false;
                self.dados_screen = DadosScreen::Cores;
            }
            DadosScreen::VinculosSelecionarCores | DadosScreen::VinculosLista => {
                self.dados_screen = DadosScreen::VinculosMenu;
            }
            DadosScreen::VinculosSelecionarTecidoCriar
            | DadosScreen::VinculosSelecionarTecidoVer => {
                self.dados_screen = DadosScreen::VinculosMenu;
            }
            DadosScreen::Tecidos | DadosScreen::Cores | DadosScreen::VinculosMenu => {
                self.dados_screen = DadosScreen::Menu;
            }
            DadosScreen::Menu => {
                self.section = Section::Dashboard;
            }
        }
    }

    fn cadastrar_tecido(&mut self) {
        if !self.tecido_form.is_valid() {
            return;
        }

        let sku = self.tecido_form.sku(&self.tecidos, self.editing_tecido_id);
        let nome = self.tecido_form.nome.trim().to_string();

        match (self.editing_tecido_id, &self.db_pool) {
            (Some(id), Some(pool)) => {
                if let Err(error) =
                    self.db_runtime
                        .block_on(db::update_tecido(pool, id, &self.tecido_form, &sku))
                {
                    self.db_status = format!("Erro ao salvar no banco: {error}");
                    return;
                }
            }
            (None, Some(pool)) => {
                if let Err(error) =
                    self.db_runtime
                        .block_on(db::insert_tecido(pool, &self.tecido_form, &sku))
                {
                    self.db_status = format!("Erro ao salvar no banco: {error}");
                    return;
                }
            }
            _ => {}
        }

        self.reload_tecidos();
        self.db_status = String::from("Tecido salvo no banco local");
        self.tecido_form = TecidoForm::default();
        self.editing_tecido_id = None;
        self.pending_delete = false;
        self.tecido_option = self
            .tecidos
            .iter()
            .position(|tecido| tecido.nome == nome)
            .map(|index| index + 1)
            .unwrap_or(0);
        self.dados_screen = DadosScreen::Tecidos;
    }

    fn open_new_tecido(&mut self) {
        self.tecido_form = TecidoForm::default();
        self.editing_tecido_id = None;
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarTecido;
    }

    fn open_edit_tecido(&mut self, index: usize) {
        let Some(tecido) = self.tecidos.get(index) else {
            return;
        };
        self.tecido_form = TecidoForm::from_record(tecido);
        self.editing_tecido_id = Some(tecido.id);
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarTecido;
    }

    fn excluir_tecido_confirmado(&mut self) {
        let Some(id) = self.editing_tecido_id else {
            self.pending_delete = false;
            return;
        };

        if let Some(pool) = &self.db_pool {
            if let Err(error) = self.db_runtime.block_on(db::delete_tecido(pool, id)) {
                self.db_status = format!("Erro ao excluir no banco: {error}");
                self.pending_delete = false;
                return;
            }
        }

        self.reload_tecidos();
        self.db_status = String::from("Tecido excluido do banco local");
        self.tecido_form = TecidoForm::default();
        self.editing_tecido_id = None;
        self.pending_delete = false;
        self.tecido_option = 0;
        self.dados_screen = DadosScreen::Tecidos;
    }

    fn open_new_cor(&mut self) {
        self.cor_form = CorForm::default();
        self.editing_cor_id = None;
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarCor;
    }

    fn open_edit_cor(&mut self, index: usize) {
        let Some(cor) = self.cores.get(index) else {
            return;
        };
        self.cor_form = CorForm::from_record(cor);
        self.editing_cor_id = Some(cor.id);
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarCor;
    }

    fn confirmar_cor(&mut self) {
        if !self.cor_form.is_valid() {
            return;
        }

        let nome = self.cor_form.nome.trim().to_string();
        if let Some(pool) = &self.db_pool {
            let result = match self.editing_cor_id {
                Some(id) => {
                    let sku = self.cor_form.sku(&self.cores, self.editing_cor_id);
                    self.db_runtime.block_on(db::update_cor(
                        pool,
                        id,
                        &self.cor_form.nome,
                        &sku,
                        &self.cor_form.hex,
                    ))
                }
                None => {
                    let sku = self.cor_form.sku(&self.cores, self.editing_cor_id);
                    self.db_runtime.block_on(db::insert_cor(
                        pool,
                        &self.cor_form.nome,
                        &sku,
                        &self.cor_form.hex,
                    ))
                }
            };
            if let Err(error) = result {
                self.db_status = format!("Erro ao salvar cor: {error}");
                return;
            }
        }

        self.reload_cores();
        self.db_status = String::from("Cor salva no banco local");
        self.cor_form = CorForm::default();
        self.editing_cor_id = None;
        self.pending_delete = false;
        self.cor_option = self
            .cores
            .iter()
            .position(|cor| cor.nome == nome)
            .map(|index| index + 1)
            .unwrap_or(0);
        self.dados_screen = DadosScreen::Cores;
    }

    fn excluir_cor_confirmada(&mut self) {
        let Some(id) = self.editing_cor_id else {
            self.pending_delete = false;
            return;
        };
        if let Some(pool) = &self.db_pool {
            if let Err(error) = self.db_runtime.block_on(db::delete_cor(pool, id)) {
                self.db_status = format!("Erro ao excluir cor: {error}");
                self.pending_delete = false;
                return;
            }
        }
        self.reload_cores();
        self.db_status = String::from("Cor excluida do banco local");
        self.cor_form = CorForm::default();
        self.editing_cor_id = None;
        self.pending_delete = false;
        self.cor_option = 0;
        self.dados_screen = DadosScreen::Cores;
    }

    fn reload_tecidos(&mut self) {
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_tecidos(pool)) {
                Ok(tecidos) => self.tecidos = tecidos,
                Err(error) => self.db_status = format!("Erro ao recarregar tecidos: {error}"),
            }
        }
    }

    fn reload_cores(&mut self) {
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_cores(pool)) {
                Ok(cores) => self.cores = cores,
                Err(error) => self.db_status = format!("Erro ao recarregar cores: {error}"),
            }
        }
    }

    fn selected_vinculo_tecido(&self) -> Option<&TecidoRecord> {
        self.tecidos.get(self.vinculo_tecido_option)
    }

    fn open_vinculo_cores(&mut self) {
        let Some(tecido_id) = self.selected_vinculo_tecido().map(|tecido| tecido.id) else {
            return;
        };
        self.load_vinculos(tecido_id);
        self.selected_vinculo_cores = self.vinculos.iter().map(|vinculo| vinculo.cor_id).collect();
        self.vinculo_criar_option = 0;
        self.dados_screen = DadosScreen::VinculosSelecionarCores;
    }

    fn open_vinculo_lista(&mut self) {
        let Some(tecido_id) = self.selected_vinculo_tecido().map(|tecido| tecido.id) else {
            return;
        };
        self.load_vinculos(tecido_id);
        self.vinculo_lista_option = 0;
        self.dados_screen = DadosScreen::VinculosLista;
    }

    fn toggle_vinculo_cor(&mut self) {
        let Some(cor) = self.cores.get(self.vinculo_criar_option) else {
            return;
        };
        if let Some(position) = self
            .selected_vinculo_cores
            .iter()
            .position(|cor_id| *cor_id == cor.id)
        {
            self.selected_vinculo_cores.remove(position);
        } else {
            self.selected_vinculo_cores.push(cor.id);
        }
    }

    fn handle_vinculo_criar_enter(&mut self) {
        if self.vinculo_criar_option < self.cores.len() {
            return;
        }

        if self.vinculo_criar_option == self.cores.len() {
            self.salvar_vinculos();
            self.dados_screen = DadosScreen::VinculosMenu;
        } else {
            self.voltar_dados();
        }
    }

    fn salvar_vinculos(&mut self) {
        let Some(tecido) = self.selected_vinculo_tecido() else {
            return;
        };
        let vinculos: Vec<(i64, String)> = self
            .selected_vinculo_cores
            .iter()
            .filter_map(|cor_id| {
                let cor = self.cores.iter().find(|cor| cor.id == *cor_id)?;
                Some((*cor_id, build_vinculo_sku(tecido, cor)))
            })
            .collect();

        if let Some(pool) = &self.db_pool {
            if let Err(error) = self
                .db_runtime
                .block_on(db::replace_vinculos(pool, tecido.id, &vinculos))
            {
                self.db_status = format!("Erro ao salvar vinculos: {error}");
                return;
            }
        }

        self.load_vinculos(tecido.id);
        self.db_status = String::from("Vinculos atualizados");
    }

    fn load_vinculos(&mut self, tecido_id: i64) {
        if let Some(pool) = &self.db_pool {
            match self
                .db_runtime
                .block_on(db::list_vinculos_by_tecido(pool, tecido_id))
            {
                Ok(vinculos) => self.vinculos = vinculos,
                Err(error) => self.db_status = format!("Erro ao carregar vinculos: {error}"),
            }
        }
    }

    fn next_venda_option(&mut self) {
        match self.vendas_screen {
            VendasScreen::Menu => self.venda_menu_option = (self.venda_menu_option + 1) % 2,
            VendasScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option = (self.venda_tecido_option + 1) % self.tecidos.len();
                }
            }
            VendasScreen::SelecionarTipo => {
                self.venda_tipo_option = (self.venda_tipo_option + 1) % 2
            }
            VendasScreen::SelecionarVinculo => {
                if !self.venda_vinculos.is_empty() {
                    self.venda_vinculo_option =
                        (self.venda_vinculo_option + 1) % self.venda_vinculos.len();
                }
            }
            VendasScreen::Lancamento => self.venda_field = self.venda_field.next(),
            VendasScreen::Historico => {}
        }
    }

    fn previous_venda_option(&mut self) {
        match self.vendas_screen {
            VendasScreen::Menu => self.venda_menu_option = (self.venda_menu_option + 1) % 2,
            VendasScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option =
                        (self.venda_tecido_option + self.tecidos.len() - 1) % self.tecidos.len();
                }
            }
            VendasScreen::SelecionarTipo => {
                self.venda_tipo_option = (self.venda_tipo_option + 1) % 2
            }
            VendasScreen::SelecionarVinculo => {
                if !self.venda_vinculos.is_empty() {
                    self.venda_vinculo_option =
                        (self.venda_vinculo_option + self.venda_vinculos.len() - 1)
                            % self.venda_vinculos.len();
                }
            }
            VendasScreen::Lancamento => self.venda_field = self.venda_field.previous(),
            VendasScreen::Historico => {}
        }
    }

    fn enter_vendas(&mut self) {
        match self.vendas_screen {
            VendasScreen::Menu => {
                if self.venda_menu_option == 0 {
                    self.vendas_screen = VendasScreen::SelecionarTecido;
                    self.venda_tecido_option = 0;
                    self.venda_itens.clear();
                } else {
                    self.vendas_screen = VendasScreen::Historico;
                }
            }
            VendasScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.vendas_screen = VendasScreen::SelecionarTipo;
                    self.venda_tipo_option = 0;
                }
            }
            VendasScreen::SelecionarTipo => self.open_venda_vinculos(),
            VendasScreen::SelecionarVinculo => {
                if !self.venda_vinculos.is_empty() {
                    self.vendas_screen = VendasScreen::Lancamento;
                    self.venda_field = VendaField::Preco;
                    self.venda_preco.clear();
                    self.venda_quantidade.clear();
                }
            }
            VendasScreen::Lancamento => {
                if self.venda_field == VendaField::Confirmar {
                    self.confirmar_lancamento();
                } else {
                    self.venda_field = self.venda_field.next();
                }
            }
            VendasScreen::Historico => {}
        }
    }

    fn open_venda_vinculos(&mut self) {
        let Some(tecido) = self.tecidos.get(self.venda_tecido_option) else {
            return;
        };
        let tipo = if self.venda_tipo_option == 0 {
            "Liso"
        } else {
            "Estampado"
        };
        if let Some(pool) = &self.db_pool {
            match self
                .db_runtime
                .block_on(db::list_vinculos_by_tecido_and_tipo(pool, tecido.id, tipo))
            {
                Ok(vinculos) => self.venda_vinculos = vinculos,
                Err(error) => self.db_status = format!("Erro ao carregar vinculos venda: {error}"),
            }
        }
        self.venda_vinculo_option = 0;
        self.vendas_screen = VendasScreen::SelecionarVinculo;
    }

    fn confirmar_lancamento(&mut self) {
        let Some(vinculo) = self.venda_vinculos.get(self.venda_vinculo_option) else {
            return;
        };
        let preco = parse_number(&self.venda_preco).unwrap_or(0.0);
        let quantidade = parse_number(&self.venda_quantidade).unwrap_or(0.0);
        if preco <= 0.0 || quantidade <= 0.0 {
            return;
        }
        self.venda_itens.push(VendaItem {
            vinculo_sku: vinculo
                .sku
                .clone()
                .unwrap_or_else(|| String::from("sem-sku")),
            descricao: format!("{} / {}", vinculo.tecido_nome, vinculo.cor_nome),
            quantidade,
            preco_unitario: preco,
        });
        self.vendas_screen = VendasScreen::SelecionarVinculo;
        self.venda_preco.clear();
        self.venda_quantidade.clear();
    }

    fn push_venda_field(&mut self, character: char) {
        match self.venda_field {
            VendaField::Preco => self.venda_preco.push(character),
            VendaField::Quantidade => self.venda_quantidade.push(character),
            VendaField::Confirmar => {}
        }
    }

    fn backspace_venda_field(&mut self) {
        match self.venda_field {
            VendaField::Preco => {
                self.venda_preco.pop();
            }
            VendaField::Quantidade => {
                self.venda_quantidade.pop();
            }
            VendaField::Confirmar => {}
        }
    }

    fn voltar_vendas(&mut self) {
        self.vendas_screen = match self.vendas_screen {
            VendasScreen::Menu => VendasScreen::Menu,
            VendasScreen::SelecionarTecido | VendasScreen::Historico => VendasScreen::Menu,
            VendasScreen::SelecionarTipo => VendasScreen::SelecionarTecido,
            VendasScreen::SelecionarVinculo => VendasScreen::SelecionarTipo,
            VendasScreen::Lancamento => VendasScreen::SelecionarVinculo,
        };
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

        render_header(frame, outer[0]);
        render_tabs(frame, outer[1], self.section);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(50), Constraint::Length(38)])
            .split(outer[2]);

        match self.section {
            Section::Vendas => render_vendas(
                frame,
                body[0],
                self.vendas_screen,
                self.venda_menu_option,
                self.venda_tecido_option,
                self.venda_tipo_option,
                self.venda_vinculo_option,
                self.venda_field,
                &self.tecidos,
                &self.venda_vinculos,
                &self.venda_preco,
                &self.venda_quantidade,
                &self.venda_itens,
            ),
            Section::Dados => render_dados(
                frame,
                body[0],
                self.dados_screen,
                self.dados_option,
                self.tecido_option,
                &self.tecidos,
                &self.tecido_form,
                self.editing_tecido_id,
                self.pending_delete,
                self.cor_option,
                &self.cores,
                &self.cor_form,
                self.editing_cor_id,
                self.vinculo_menu_option,
                self.vinculo_tecido_option,
                self.vinculo_criar_option,
                self.vinculo_lista_option,
                &self.vinculos,
                &self.selected_vinculo_cores,
            ),
            section => render_content(frame, body[0], section),
        }

        render_chat(frame, body[1], &self.chat, self.focus, &self.active_skill());
        render_footer(frame, outer[3], &self.db_status, self.focus);
    }

    fn active_skill(&self) -> agent::SkillContext {
        agent::active_skill(self.section, self.dados_screen, self.dados_option)
    }
}

fn render_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("RAZAI TUI", Style::default().fg(Color::White).bold()),
        Span::raw("  Sistema de loja via terminal"),
    ]))
    .block(Block::default().borders(Borders::BOTTOM))
    .alignment(Alignment::Center);

    frame.render_widget(header, area);
}

fn render_tabs(frame: &mut Frame, area: Rect, selected: Section) {
    let titles = Section::ALL.map(|section| section.title());
    let tabs = Tabs::new(titles)
        .select(selected.index())
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());

    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, area: Rect, selected: Section) {
    let content = Paragraph::new("")
        .block(
            Block::default()
                .title(selected.title())
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    frame.render_widget(content, area);
}

fn render_vendas(
    frame: &mut Frame,
    area: Rect,
    screen: VendasScreen,
    menu_option: usize,
    tecido_option: usize,
    tipo_option: usize,
    vinculo_option: usize,
    field: VendaField,
    tecidos: &[TecidoRecord],
    vinculos: &[VinculoRecord],
    preco: &str,
    quantidade: &str,
    itens: &[VendaItem],
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(48), Constraint::Length(38)])
        .split(area);

    match screen {
        VendasScreen::Menu => render_vendas_menu(frame, chunks[0], menu_option),
        VendasScreen::SelecionarTecido => {
            render_venda_tecidos(frame, chunks[0], tecido_option, tecidos)
        }
        VendasScreen::SelecionarTipo => render_venda_tipo(frame, chunks[0], tipo_option),
        VendasScreen::SelecionarVinculo => {
            render_venda_vinculos(frame, chunks[0], vinculo_option, vinculos)
        }
        VendasScreen::Lancamento => {
            render_venda_lancamento(frame, chunks[0], field, preco, quantidade)
        }
        VendasScreen::Historico => {
            let widget = Paragraph::new("Historico de vendas ainda nao implementado.").block(
                Block::default()
                    .title("Vendas > Historico")
                    .borders(Borders::ALL),
            );
            frame.render_widget(widget, chunks[0]);
        }
    }

    render_resumo_pedido(frame, chunks[1], itens);
}

fn render_vendas_menu(frame: &mut Frame, area: Rect, selected: usize) {
    let items = ["[Nova Venda]", "[Historico de Vendas]"]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(Block::default().title("Vendas").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_venda_tecidos(frame: &mut Frame, area: Rect, selected: usize, tecidos: &[TecidoRecord]) {
    let items = tecidos
        .iter()
        .enumerate()
        .map(|(index, tecido)| ListItem::new(format!("{}. {}", index + 1, tecido.nome)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Vendas > Nova Venda > Tecido")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_venda_tipo(frame: &mut Frame, area: Rect, selected: usize) {
    let items = ["Cor", "Estampa"]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Vendas > Nova Venda > Tipo")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_venda_vinculos(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    vinculos: &[VinculoRecord],
) {
    let items = vinculos.iter().enumerate().map(|(index, vinculo)| {
        let sku = vinculo.sku.as_deref().unwrap_or("sem-sku");
        let hex = vinculo.cor_hex.as_deref().unwrap_or("#");
        ListItem::new(Line::from(vec![
            Span::raw(format!("{}. {} - ", index + 1, sku)),
            color_swatch(hex),
            Span::raw(format!(" {}", vinculo.cor_nome)),
        ]))
    });
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Vendas > Nova Venda > Vinculo")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_venda_lancamento(
    frame: &mut Frame,
    area: Rect,
    field: VendaField,
    preco: &str,
    quantidade: &str,
) {
    let valid =
        parse_number(preco).unwrap_or(0.0) > 0.0 && parse_number(quantidade).unwrap_or(0.0) > 0.0;
    let lines = vec![
        format_venda_field(VendaField::Preco, field, "Preco Unitario", preco),
        format_venda_field(VendaField::Quantidade, field, "Lancar", quantidade),
        Line::from(""),
        format_venda_action(field, valid),
    ];
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Vendas > Nova Venda > Lancamento")
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
}

fn render_resumo_pedido(frame: &mut Frame, area: Rect, itens: &[VendaItem]) {
    let mut lines = itens
        .iter()
        .map(|item| {
            Line::from(format!(
                "{} - {} | {:.2} x {:.2} = {:.2}",
                item.vinculo_sku,
                item.descricao,
                item.quantidade,
                item.preco_unitario,
                item.total()
            ))
        })
        .collect::<Vec<_>>();
    let total = itens.iter().map(VendaItem::total).sum::<f64>();
    lines.push(Line::from(""));
    lines.push(Line::from(format!("Total: {:.2}", total)));
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Resumo do pedido")
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
}

fn format_venda_field(
    field: VendaField,
    selected: VendaField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let value = if value.is_empty() { "_" } else { value };
    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
    ])
}

fn format_venda_action(selected: VendaField, valid: bool) -> Line<'static> {
    let marker = if selected == VendaField::Confirmar {
        ">"
    } else {
        " "
    };
    let suffix = if valid {
        ""
    } else {
        "  preco e quantidade obrigatorios"
    };
    Line::from(vec![
        Span::styled(
            format!("{marker} "),
            selected_style(selected == VendaField::Confirmar),
        ),
        Span::raw("[Lancar]"),
        Span::raw(suffix),
    ])
}

fn render_dados(
    frame: &mut Frame,
    area: Rect,
    screen: DadosScreen,
    selected: DadosOption,
    tecido_option: usize,
    tecidos: &[TecidoRecord],
    tecido_form: &TecidoForm,
    editing_tecido_id: Option<i64>,
    pending_delete: bool,
    cor_option: usize,
    cores: &[CorRecord],
    cor_form: &CorForm,
    editing_cor_id: Option<i64>,
    vinculo_menu_option: usize,
    vinculo_tecido_option: usize,
    vinculo_criar_option: usize,
    vinculo_lista_option: usize,
    vinculos: &[VinculoRecord],
    selected_vinculo_cores: &[i64],
) {
    match screen {
        DadosScreen::Menu => render_dados_menu(frame, area, selected),
        DadosScreen::Tecidos => render_tecidos(frame, area, tecido_option, tecidos),
        DadosScreen::CadastrarTecido => render_cadastrar_tecido(
            frame,
            area,
            tecido_form,
            tecidos,
            editing_tecido_id,
            pending_delete,
        ),
        DadosScreen::Cores => render_cores(frame, area, cor_option, cores),
        DadosScreen::CadastrarCor => {
            render_cadastrar_cor(frame, area, cor_form, cores, editing_cor_id, pending_delete)
        }
        DadosScreen::VinculosMenu => render_vinculos_menu(frame, area, vinculo_menu_option),
        DadosScreen::VinculosSelecionarTecidoCriar => render_vinculo_tecidos(
            frame,
            area,
            "Dados > Vinculos > Criar > Selecione o tecido",
            vinculo_tecido_option,
            tecidos,
        ),
        DadosScreen::VinculosSelecionarTecidoVer => render_vinculo_tecidos(
            frame,
            area,
            "Dados > Vinculos > Ver > Selecione o tecido",
            vinculo_tecido_option,
            tecidos,
        ),
        DadosScreen::VinculosSelecionarCores => render_vinculo_cores(
            frame,
            area,
            vinculo_criar_option,
            cores,
            selected_vinculo_cores,
        ),
        DadosScreen::VinculosLista => {
            render_vinculos_lista(frame, area, vinculo_lista_option, vinculos)
        }
    }
}

fn render_dados_menu(frame: &mut Frame, area: Rect, selected: DadosOption) {
    let items = DadosOption::ALL
        .iter()
        .enumerate()
        .map(|(index, option)| ListItem::new(format!("{}. {}", index + 1, option.title())));
    let mut state = ListState::default().with_selected(Some(selected.index()));
    let list = List::new(items)
        .block(Block::default().title("Dados").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_tecidos(frame: &mut Frame, area: Rect, selected: usize, tecidos: &[TecidoRecord]) {
    let items = std::iter::once(String::from("[Cadastrar tecido]"))
        .chain(tecidos.iter().map(|tecido| tecido.nome.clone()))
        .enumerate()
        .map(|(index, tecido)| ListItem::new(format!("{}. {}", index + 1, tecido)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Tecido")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_cores(frame: &mut Frame, area: Rect, selected: usize, cores: &[CorRecord]) {
    let items = std::iter::once(ListItem::new("1. [Cadastrar Cor]")).chain(
        cores.iter().enumerate().map(|(index, cor)| {
            let hex = cor.codigo_hex.as_deref().unwrap_or("#");
            let sku = cor.sku.as_deref().unwrap_or("____-__");
            ListItem::new(Line::from(vec![
                Span::raw(format!("{}. {} - ", index + 2, sku)),
                color_swatch(hex),
                Span::raw(format!(" {} ({hex})", cor.nome)),
            ]))
        }),
    );
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Cores")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculos_menu(frame: &mut Frame, area: Rect, selected: usize) {
    let items = ["[Criar Vinculos]", "[Ver Vinculos]"]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Vinculos")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculo_tecidos(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    selected: usize,
    tecidos: &[TecidoRecord],
) {
    let items = tecidos
        .iter()
        .enumerate()
        .map(|(index, tecido)| ListItem::new(format!("{}. {}", index + 1, tecido.nome)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculo_cores(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    cores: &[CorRecord],
    selected_cores: &[i64],
) {
    let items = cores
        .iter()
        .enumerate()
        .map(|(index, cor)| {
            let marker = if selected_cores.contains(&cor.id) {
                "[+]"
            } else {
                "[ ]"
            };
            let sku = cor.sku.as_deref().unwrap_or("____-__");
            let hex = cor.codigo_hex.as_deref().unwrap_or("#");
            ListItem::new(Line::from(vec![
                Span::raw(format!("{}. {} {} - ", index + 1, marker, sku)),
                color_swatch(hex),
                Span::raw(format!(" {}", cor.nome)),
            ]))
        })
        .chain([ListItem::new("[Confirmar]"), ListItem::new("[Voltar]")]);
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Vinculos > Criar > Selecione as cores")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculos_lista(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    vinculos: &[VinculoRecord],
) {
    let items = vinculos.iter().enumerate().map(|(index, vinculo)| {
        let sku = vinculo.sku.as_deref().unwrap_or("sem-sku");
        let hex = vinculo.cor_hex.as_deref().unwrap_or("#");
        ListItem::new(Line::from(vec![
            Span::raw(format!(
                "{}. {} - {} / ",
                index + 1,
                sku,
                vinculo.tecido_nome
            )),
            color_swatch(hex),
            Span::raw(format!(" {}", vinculo.cor_nome)),
        ]))
    });
    let mut state =
        ListState::default().with_selected(Some(selected.min(vinculos.len().saturating_sub(1))));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Vinculos > Cores vinculadas")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_cadastrar_cor(
    frame: &mut Frame,
    area: Rect,
    form: &CorForm,
    cores: &[CorRecord],
    editing_id: Option<i64>,
    pending_delete: bool,
) {
    let lines = vec![
        format_cor_hex_field(CorField::Hex, form.selected_field, "Hex", &form.hex),
        format_cor_field(
            CorField::Nome,
            form.selected_field,
            "Nome da Cor",
            &form.nome,
        ),
        Line::from(""),
        format_cor_action(
            CorField::Confirmar,
            form.selected_field,
            "[Confirmar]",
            form.is_valid(),
        ),
        format_cor_action(CorField::Voltar, form.selected_field, "[Voltar]", true),
        if editing_id.is_some() {
            format_cor_action(CorField::Excluir, form.selected_field, "[Excluir]", true)
        } else {
            Line::from("")
        },
    ];
    let form_widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Dados > Cores > Cadastrar Cor")
            .borders(Borders::ALL),
    );
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(22)])
        .split(area);
    let sku = Paragraph::new(form.sku(cores, editing_id))
        .block(Block::default().title("SKU").borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(form_widget, chunks[0]);
    frame.render_widget(sku, chunks[1]);

    if pending_delete {
        render_confirm_dialog(frame, area, "Excluir esta cor?");
    }
}

fn render_cadastrar_tecido(
    frame: &mut Frame,
    area: Rect,
    form: &TecidoForm,
    tecidos: &[TecidoRecord],
    editing_tecido_id: Option<i64>,
    pending_delete: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(22)])
        .split(area);
    let calculated = form.calculated_values();
    let fields = vec![
        format_field(
            TecidoField::Nome,
            form.selected_field,
            "Nome *",
            &form.nome,
            None,
        ),
        format_field(
            TecidoField::Composicao,
            form.selected_field,
            "Composicao *",
            &form.composicao,
            None,
        ),
        format_field(
            TecidoField::Largura,
            form.selected_field,
            "Largura *",
            &form.largura,
            None,
        ),
        format_select(
            TecidoField::Tipo,
            form.selected_field,
            "Tipo",
            form.tipo.value(TIPO_OPTIONS),
        ),
        format_select(
            TecidoField::Transparencia,
            form.selected_field,
            "Transparencia",
            form.transparencia.value(NIVEL_OPTIONS),
        ),
        format_select(
            TecidoField::Elasticidade,
            form.selected_field,
            "Elasticidade",
            form.elasticidade.value(NIVEL_OPTIONS),
        ),
        format_select(
            TecidoField::Acabamento,
            form.selected_field,
            "Acabamento",
            form.acabamento.value(ACABAMENTO_OPTIONS),
        ),
        format_field(
            TecidoField::Rendimento,
            form.selected_field,
            "Rendimento m/kg",
            &form.rendimento,
            calculated.rendimento,
        ),
        format_field(
            TecidoField::GramaturaLinear,
            form.selected_field,
            "Gramatura Linear g/m",
            &form.gramatura_linear,
            calculated.gramatura_linear,
        ),
        format_field(
            TecidoField::GramaturaM2,
            form.selected_field,
            "Gramatura g/m2",
            &form.gramatura_m2,
            calculated.gramatura_m2,
        ),
        Line::from(""),
        format_submit(
            form.selected_field,
            form.is_valid(),
            editing_tecido_id.is_some(),
        ),
        format_tecido_action(TecidoField::Voltar, form.selected_field, "[Voltar]"),
        format_delete(form.selected_field, editing_tecido_id.is_some()),
    ];
    let form_widget = Paragraph::new(Text::from(fields))
        .block(
            Block::default()
                .title("Dados > Tecido > Cadastrar tecido")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left);
    let sku = Paragraph::new(form.sku(tecidos, editing_tecido_id))
        .block(Block::default().title("SKU").borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(form_widget, chunks[0]);
    frame.render_widget(sku, chunks[1]);

    if pending_delete {
        render_confirm_dialog(frame, area, "Excluir este tecido?");
    }
}

fn render_chat(
    frame: &mut Frame,
    area: Rect,
    chat: &ChatState,
    focus: Focus,
    skill: &agent::SkillContext,
) {
    let border_style = Style::default().fg(Color::White);
    let selected_border_style = if focus == Focus::Chat {
        Style::default().fg(Color::Cyan)
    } else {
        border_style
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    let skill_panel = Paragraph::new(format!("Skill: {}\n{}", skill.name, skill.description))
        .block(
            Block::default()
                .title("Agente")
                .borders(Borders::ALL)
                .border_style(selected_border_style),
        );
    frame.render_widget(skill_panel, chunks[0]);

    let history = if chat.messages.is_empty() {
        String::from("F2 foca o chat. Enter envia.")
    } else {
        chat.messages
            .iter()
            .rev()
            .take(8)
            .rev()
            .map(|message| format!("{}: {}", message.author, message.text))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let messages = Paragraph::new(history).block(
        Block::default()
            .title("Chat")
            .borders(Borders::ALL)
            .border_style(selected_border_style),
    );
    frame.render_widget(messages, chunks[1]);

    let input = Paragraph::new(Span::styled(
        chat.input.clone(),
        Style::default().fg(Color::Yellow),
    ))
    .block(
        Block::default()
            .title("Mensagem")
            .borders(Borders::ALL)
            .border_style(selected_border_style),
    );
    frame.render_widget(input, chunks[2]);
}

fn format_field(
    field: TecidoField,
    selected: TecidoField,
    label: &str,
    value: &str,
    calculated: Option<f64>,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let value = if value.is_empty() { "_" } else { value };
    let mut spans = vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
    ];

    if let Some(number) = calculated {
        let calculated = match field {
            TecidoField::GramaturaLinear | TecidoField::GramaturaM2 => {
                format!("  calculado: {}", round_to_nearest_ten(number))
            }
            _ => format!("  calculado: {:.2}", number),
        };
        spans.push(Span::raw(calculated));
    }

    Line::from(spans)
}

fn format_select(
    field: TecidoField,
    selected: TecidoField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };

    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(format!("[{value}]"), Style::default().fg(Color::Yellow)),
    ])
}

fn format_cor_field(
    field: CorField,
    selected: CorField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let value = if value.is_empty() {
        if field == CorField::Hex { "#" } else { "_" }
    } else {
        value
    };

    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
    ])
}

fn format_cor_hex_field(
    field: CorField,
    selected: CorField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let value = if value.is_empty() { "#" } else { value };

    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        color_swatch(value),
        if parse_hex_color(value).is_some() {
            Span::raw("")
        } else {
            Span::raw("  hex invalido, use #RRGGBB")
        },
    ])
}

fn format_cor_action(
    field: CorField,
    selected: CorField,
    label: &str,
    enabled: bool,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let suffix = if enabled {
        ""
    } else {
        "  campos obrigatorios pendentes"
    };

    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(label.to_string()),
        Span::raw(suffix),
    ])
}

fn format_submit(selected: TecidoField, valid: bool, editing: bool) -> Line<'static> {
    let marker = if selected == TecidoField::Salvar {
        ">"
    } else {
        " "
    };
    let status = if valid {
        ""
    } else {
        "  campos obrigatorios pendentes"
    };

    Line::from(vec![
        Span::styled(
            format!("{marker} "),
            selected_style(selected == TecidoField::Salvar),
        ),
        Span::raw(if editing {
            "[Confirmar]"
        } else {
            "[Confirmar]"
        }),
        Span::raw(status),
    ])
}

fn format_delete(selected: TecidoField, editing: bool) -> Line<'static> {
    if !editing {
        return Line::from("");
    }

    let marker = if selected == TecidoField::Excluir {
        ">"
    } else {
        " "
    };

    Line::from(vec![
        Span::styled(
            format!("{marker} "),
            selected_style(selected == TecidoField::Excluir),
        ),
        Span::raw("[Excluir]"),
    ])
}

fn format_tecido_action(field: TecidoField, selected: TecidoField, label: &str) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(label.to_string()),
    ])
}

fn render_confirm_dialog(frame: &mut Frame, area: Rect, message: &str) {
    let popup_area = centered_rect(54, 7, area);
    let dialog = Paragraph::new(format!("{message}\n\nS = confirmar   N/Esc = cancelar"))
        .block(
            Block::default()
                .title("Confirmacao destrutiva")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(dialog, popup_area);
}

fn render_footer(frame: &mut Frame, area: Rect, db_status: &str, focus: Focus) {
    let footer = Paragraph::new(format!(
        "Foco: {} | F2 chat | Cima/Baixo selecionar | Space marcar/desmarcar | Esq/Dir alterar select | Enter abrir/confirmar | Backspace apagar | Esc voltar/cancelar | Ctrl+C sair | {db_status}",
        focus.title(),
    ))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}
