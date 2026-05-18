use crossterm::event::KeyCode;

use super::App;
use crate::{db, models::*};

impl App {
    pub(super) fn handle_vendas_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.voltar_vendas(),
            KeyCode::Backspace => self.backspace_venda_field(),
            KeyCode::Up => self.previous_venda_option(),
            KeyCode::Down => self.next_venda_option(),
            KeyCode::Left | KeyCode::BackTab => self.section = self.section.previous(),
            KeyCode::Right | KeyCode::Tab => self.section = self.section.next(),
            KeyCode::Enter => self.enter_vendas(),
            KeyCode::Char(character)
                if !character.is_control() && self.vendas_screen == VendasScreen::Lancamento =>
            {
                self.push_venda_field(character);
            }
            _ => {}
        }
    }

    pub(super) fn next_venda_option(&mut self) {
        match self.vendas_screen {
            VendasScreen::Menu => self.venda_menu_option = (self.venda_menu_option + 1) % 2,
            VendasScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option = (self.venda_tecido_option + 1) % self.tecidos.len();
                }
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

    pub(super) fn previous_venda_option(&mut self) {
        match self.vendas_screen {
            VendasScreen::Menu => self.venda_menu_option = (self.venda_menu_option + 1) % 2,
            VendasScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option =
                        (self.venda_tecido_option + self.tecidos.len() - 1) % self.tecidos.len();
                }
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

    pub(super) fn enter_vendas(&mut self) {
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
                    self.open_venda_vinculos();
                }
            }
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

    pub(super) fn open_venda_vinculos(&mut self) {
        let Some(tecido) = self.tecidos.get(self.venda_tecido_option) else {
            return;
        };
        let tipo = if tecido.tipo == "Estampado" {
            "Estampado"
        } else {
            "Liso"
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

    pub(super) fn confirmar_lancamento(&mut self) {
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

    pub(super) fn push_venda_field(&mut self, character: char) {
        match self.venda_field {
            VendaField::Preco => self.venda_preco.push(character),
            VendaField::Quantidade => self.venda_quantidade.push(character),
            VendaField::Confirmar => {}
        }
    }

    pub(super) fn backspace_venda_field(&mut self) {
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

    pub(super) fn voltar_vendas(&mut self) {
        self.vendas_screen = match self.vendas_screen {
            VendasScreen::Menu => VendasScreen::Menu,
            VendasScreen::SelecionarTecido | VendasScreen::Historico => VendasScreen::Menu,
            VendasScreen::SelecionarVinculo => VendasScreen::SelecionarTecido,
            VendasScreen::Lancamento => VendasScreen::SelecionarVinculo,
        };
    }
}
