use crate::app::App;
use crate::models::DadosScreen;

impl App {
    pub(in crate::app) fn next_tecido(&mut self) {
        self.tecido_option = (self.tecido_option + 1) % self.tecidos_menu_len();
    }

    pub(in crate::app) fn previous_tecido(&mut self) {
        self.tecido_option =
            (self.tecido_option + self.tecidos_menu_len() - 1) % self.tecidos_menu_len();
    }

    pub(in crate::app) fn tecidos_menu_len(&self) -> usize {
        self.tecidos.len() + 1
    }

    pub(in crate::app) fn next_cor(&mut self) {
        self.cor_option = (self.cor_option + 1) % self.cor_menu_len();
    }

    pub(in crate::app) fn previous_cor(&mut self) {
        let len = self.cor_menu_len();
        self.cor_option = (self.cor_option + len - 1) % len;
    }

    pub(in crate::app) fn cor_menu_len(&self) -> usize {
        match self.dados_screen {
            DadosScreen::Estampas => self.estampas.len() + 1,
            _ => self.cores.len() + 1,
        }
    }

    pub(in crate::app) fn next_vinculo_menu(&mut self) {
        self.vinculo_menu_option = (self.vinculo_menu_option + 1) % 2;
    }

    pub(in crate::app) fn previous_vinculo_menu(&mut self) {
        self.vinculo_menu_option = (self.vinculo_menu_option + 1) % 2;
    }

    pub(in crate::app) fn next_vinculo_tecido(&mut self) {
        if !self.tecidos.is_empty() {
            self.vinculo_tecido_option = (self.vinculo_tecido_option + 1) % self.tecidos.len();
        }
    }

    pub(in crate::app) fn previous_vinculo_tecido(&mut self) {
        if !self.tecidos.is_empty() {
            self.vinculo_tecido_option =
                (self.vinculo_tecido_option + self.tecidos.len() - 1) % self.tecidos.len();
        }
    }

    pub(in crate::app) fn vinculo_criar_len(&self) -> usize {
        self.vinculo_item_len() + 2
    }

    pub(in crate::app) fn vinculo_item_len(&self) -> usize {
        if self.selected_vinculo_usa_estampas() {
            self.estampas.len()
        } else {
            self.cores.len()
        }
    }

    pub(in crate::app) fn selected_vinculo_usa_estampas(&self) -> bool {
        self.selected_vinculo_tecido()
            .map(|tecido| tecido.tipo == "Estampado")
            .unwrap_or(false)
    }

    pub(in crate::app) fn next_vinculo_criar_option(&mut self) {
        self.vinculo_criar_option = (self.vinculo_criar_option + 1) % self.vinculo_criar_len();
    }

    pub(in crate::app) fn previous_vinculo_criar_option(&mut self) {
        self.vinculo_criar_option =
            (self.vinculo_criar_option + self.vinculo_criar_len() - 1) % self.vinculo_criar_len();
    }

    pub(in crate::app) fn next_vinculo_lista(&mut self) {
        if !self.vinculos.is_empty() {
            self.vinculo_lista_option = (self.vinculo_lista_option + 1) % self.vinculos.len();
        }
    }

    pub(in crate::app) fn previous_vinculo_lista(&mut self) {
        if !self.vinculos.is_empty() {
            self.vinculo_lista_option =
                (self.vinculo_lista_option + self.vinculos.len() - 1) % self.vinculos.len();
        }
    }
}
