use crossterm::event::KeyCode;

use super::App;
use crate::{db, models::*};

mod history;
mod receipt;

impl App {
    pub(super) fn handle_vendas_key(&mut self, key: KeyCode) {
        if self.pending_delete_venda_item {
            match key {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.pending_delete_venda_item = false;
                }
                KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.excluir_venda_item_confirmado();
                }
                _ => {}
            }
            return;
        }

        if self.pending_delete_venda {
            match key {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.pending_delete_venda = false;
                }
                KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.excluir_venda_confirmada();
                }
                _ => {}
            }
            return;
        }

        if self.finalizar_venda_dialog {
            match key {
                KeyCode::Esc => self.finalizar_venda_dialog = false,
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down | KeyCode::Tab => {
                    self.finalizar_venda_option = self.finalizar_venda_option.next();
                }
                KeyCode::BackTab => {
                    self.finalizar_venda_option = self.finalizar_venda_option.previous();
                }
                KeyCode::Enter => self.confirmar_finalizacao_venda(),
                _ => {}
            }
            return;
        }

        if self.vendas_screen == VendasScreen::Lancamento && self.venda_dropdown.is_some() {
            match key {
                KeyCode::Esc | KeyCode::Enter => self.venda_dropdown = None,
                KeyCode::Up => self.previous_venda_dropdown_option(),
                KeyCode::Down => self.next_venda_dropdown_option(),
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Esc => self.voltar_vendas(),
            KeyCode::Backspace => self.backspace_venda_field(),
            KeyCode::Delete => self.delete_venda_item(),
            KeyCode::Up => self.previous_venda_option(),
            KeyCode::Down => self.next_venda_option(),
            KeyCode::Left => self.section = self.section.previous(),
            KeyCode::Right => self.section = self.section.next(),
            KeyCode::Enter => self.enter_vendas(),
            KeyCode::Char(character)
                if !character.is_control() && self.vendas_screen == VendasScreen::Lancamento =>
            {
                self.push_venda_field(character);
            }
            KeyCode::Char(character)
                if !character.is_control() && self.vendas_screen == VendasScreen::Historico =>
            {
                self.push_venda_historico_field(character);
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
            VendasScreen::Lancamento => {
                if self.venda_resumo_focus {
                    self.next_venda_item();
                    return;
                }
                self.venda_field = self.venda_field.next();
                self.normalize_venda_field();
            }
            VendasScreen::Historico => {
                if self.venda_historico_field < 2 {
                    self.venda_historico_field += 1;
                } else if !self.vendas_historico.is_empty() {
                    self.venda_historico_option =
                        (self.venda_historico_option + 1) % self.vendas_historico.len();
                }
            }
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
            VendasScreen::Lancamento => {
                if self.venda_resumo_focus {
                    self.previous_venda_item();
                    return;
                }
                self.venda_field = self.venda_field.previous();
                self.normalize_venda_field();
            }
            VendasScreen::Historico => {
                if self.venda_historico_field == 2 && self.venda_historico_option > 0 {
                    self.venda_historico_option =
                        (self.venda_historico_option + self.vendas_historico.len() - 1)
                            % self.vendas_historico.len();
                } else if self.venda_historico_field > 0 {
                    self.venda_historico_field -= 1;
                }
            }
        }
    }

    pub(super) fn enter_vendas(&mut self) {
        match self.vendas_screen {
            VendasScreen::Menu => {
                self.venda_resumo_focus = false;
                if self.venda_menu_option == 0 {
                    self.vendas_screen = VendasScreen::SelecionarTecido;
                    self.venda_tecido_option = 0;
                    self.venda_itens.clear();
                    self.reset_venda_item_editing();
                    self.editing_venda_id = None;
                    self.pending_delete_venda = false;
                } else {
                    self.venda_historico_field = 2;
                    self.venda_historico_option = 0;
                    self.reload_vendas_historico();
                    self.vendas_screen = VendasScreen::Historico;
                }
            }
            VendasScreen::SelecionarTecido => {
                self.venda_resumo_focus = false;
                if !self.tecidos.is_empty() {
                    self.open_venda_vinculos();
                }
            }
            VendasScreen::SelecionarVinculo => {
                self.venda_resumo_focus = false;
                if !self.venda_vinculos.is_empty() {
                    self.vendas_screen = VendasScreen::Lancamento;
                    self.venda_field = VendaField::Preco;
                    self.venda_dropdown = None;
                    self.venda_preco.clear();
                    self.venda_quantidade.clear();
                }
            }
            VendasScreen::Lancamento => {
                if self.venda_resumo_focus {
                    self.editar_venda_item_selecionado();
                    return;
                }
                if matches!(self.venda_field, VendaField::Tecido | VendaField::Vinculo) {
                    self.venda_dropdown = Some(self.venda_field);
                    return;
                }
                if self.venda_field == VendaField::Quantidade {
                    self.confirmar_lancamento();
                } else if self.venda_field == VendaField::Finalizar {
                    self.abrir_dialog_finalizar_venda();
                } else if self.venda_field == VendaField::Cancelar {
                    self.cancelar_venda();
                } else if self.venda_field == VendaField::Excluir {
                    self.pending_delete_venda = self.editing_venda_id.is_some();
                } else {
                    self.venda_field = self.venda_field.next();
                }
            }
            VendasScreen::Historico => {
                self.venda_resumo_focus = false;
                if self.venda_historico_field < 2 {
                    self.reload_vendas_historico();
                } else {
                    self.open_edit_venda();
                }
            }
        }
    }

    pub(super) fn open_venda_vinculos(&mut self) {
        let Some(tecido) = self.tecidos.get(self.venda_tecido_option) else {
            self.venda_vinculos.clear();
            return;
        };
        let usa_estampas = tecido.tipo == "Estampado";
        if let Some(pool) = &self.db_pool {
            let result = if usa_estampas {
                self.db_runtime
                    .block_on(db::list_estampa_vinculos_by_tecido(pool, tecido.id))
            } else {
                self.db_runtime
                    .block_on(db::list_vinculos_by_tecido(pool, tecido.id))
            };
            match result {
                Ok(vinculos) => {
                    self.venda_vinculos = vinculos;
                    self.db_status = if usa_estampas {
                        String::from("Venda: listando estampas vinculadas ao tecido")
                    } else {
                        String::from("Venda: listando cores vinculadas ao tecido")
                    };
                }
                Err(error) => self.db_status = format!("Erro ao carregar vinculos venda: {error}"),
            }
        } else {
            self.venda_vinculos.clear();
        }
        self.venda_vinculo_option = 0;
        self.vendas_screen = VendasScreen::SelecionarVinculo;
    }

    pub(super) fn reload_venda_vinculos_for_current_tecido(&mut self) {
        let current_screen = self.vendas_screen;
        self.open_venda_vinculos();
        self.vendas_screen = current_screen;
    }

    pub(super) fn next_venda_dropdown_option(&mut self) {
        match self.venda_dropdown {
            Some(VendaField::Tecido) => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option = (self.venda_tecido_option + 1) % self.tecidos.len();
                    self.editing_venda_item_descricao = None;
                    self.reload_venda_vinculos_for_current_tecido();
                }
            }
            Some(VendaField::Vinculo) => {
                if !self.venda_vinculos.is_empty() {
                    self.venda_vinculo_option =
                        (self.venda_vinculo_option + 1) % self.venda_vinculos.len();
                    self.editing_venda_item_descricao = None;
                }
            }
            _ => {}
        }
    }

    pub(super) fn previous_venda_dropdown_option(&mut self) {
        match self.venda_dropdown {
            Some(VendaField::Tecido) => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option =
                        (self.venda_tecido_option + self.tecidos.len() - 1) % self.tecidos.len();
                    self.editing_venda_item_descricao = None;
                    self.reload_venda_vinculos_for_current_tecido();
                }
            }
            Some(VendaField::Vinculo) => {
                if !self.venda_vinculos.is_empty() {
                    self.venda_vinculo_option =
                        (self.venda_vinculo_option + self.venda_vinculos.len() - 1)
                            % self.venda_vinculos.len();
                    self.editing_venda_item_descricao = None;
                }
            }
            _ => {}
        }
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
        let descricao = self
            .editing_venda_item_descricao
            .clone()
            .unwrap_or_else(|| format!("{} - {}", vinculo.tecido_nome, vinculo.cor_nome));
        let item = VendaItem {
            descricao,
            quantidade,
            preco_unitario: preco,
        };
        if let Some(index) = self.editing_venda_item {
            if let Some(current) = self.venda_itens.get_mut(index) {
                *current = item;
                self.venda_item_option = index;
                self.db_status = format!("Lancamento {} atualizado", index + 1);
            }
        } else {
            self.venda_itens.push(item);
            self.venda_item_option = self.venda_itens.len().saturating_sub(1);
            self.db_status = String::from("Lancamento adicionado ao resumo");
        }
        self.venda_dropdown = None;
        self.venda_quantidade.clear();
        self.editing_venda_item = None;
        self.editing_venda_item_descricao = None;
    }

    pub(super) fn abrir_dialog_finalizar_venda(&mut self) {
        if self.venda_itens.is_empty() {
            self.db_status = String::from("Adicione ao menos um item antes de finalizar");
            return;
        }

        self.finalizar_venda_dialog = true;
        self.finalizar_venda_option = FinalizarVendaOption::Finalizar;
    }

    pub(super) fn confirmar_finalizacao_venda(&mut self) {
        let imprimir = self.finalizar_venda_option == FinalizarVendaOption::FinalizarEImprimir;
        if let Some(pool) = &self.db_pool {
            let result = if let Some(venda_id) = self.editing_venda_id {
                self.db_runtime
                    .block_on(db::update_venda(pool, venda_id, &self.venda_itens))
            } else {
                self.db_runtime
                    .block_on(db::insert_venda(pool, &self.venda_itens))
                    .map(|_| ())
            };
            if let Err(error) = result {
                self.db_status = format!("Erro ao salvar venda: {error}");
                return;
            }
            self.reload_vendas_historico();
        }

        self.db_status = if self.editing_venda_id.is_some() {
            String::from("Venda atualizada no historico")
        } else {
            String::from("Venda registrada no historico")
        };
        if imprimir {
            self.db_status = match &self.selected_printer {
                Some(printer) => match self.imprimir_recibo(printer) {
                    Ok(()) => format!("Venda salva. Recibo enviado para {printer}"),
                    Err(error) => format!("Venda salva. Erro ao imprimir: {error}"),
                },
                None => String::from("Venda salva. Configure uma impressora para imprimir"),
            };
        }
        self.finalizar_venda_dialog = false;
        self.vendas_screen = VendasScreen::Menu;
        self.venda_dropdown = None;
        self.venda_preco.clear();
        self.venda_quantidade.clear();
        self.venda_itens.clear();
        self.reset_venda_item_editing();
        self.editing_venda_id = None;
        self.pending_delete_venda = false;
    }

    pub(super) fn cancelar_venda(&mut self) {
        self.venda_itens.clear();
        self.reset_venda_item_editing();
        self.venda_vinculos.clear();
        self.finalizar_venda_dialog = false;
        self.venda_dropdown = None;
        self.venda_preco.clear();
        self.venda_quantidade.clear();
        self.venda_tecido_option = 0;
        self.venda_vinculo_option = 0;
        self.vendas_screen = VendasScreen::Menu;
        self.editing_venda_id = None;
        self.pending_delete_venda = false;
        self.db_status = String::from("Venda cancelada");
    }

    pub(super) fn excluir_venda_confirmada(&mut self) {
        let Some(venda_id) = self.editing_venda_id else {
            self.pending_delete_venda = false;
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para excluir venda");
            self.pending_delete_venda = false;
            return;
        };

        match self.db_runtime.block_on(db::delete_venda(pool, venda_id)) {
            Ok(()) => {
                self.reload_vendas_historico();
                self.venda_historico_option = self
                    .venda_historico_option
                    .min(self.vendas_historico.len().saturating_sub(1));
                self.venda_itens.clear();
                self.reset_venda_item_editing();
                self.venda_vinculos.clear();
                self.venda_preco.clear();
                self.venda_quantidade.clear();
                self.venda_dropdown = None;
                self.editing_venda_id = None;
                self.pending_delete_venda = false;
                self.finalizar_venda_dialog = false;
                self.vendas_screen = VendasScreen::Historico;
                self.db_status = format!("Venda #{venda_id} excluida");
            }
            Err(error) => {
                self.pending_delete_venda = false;
                self.db_status = format!("Erro ao excluir venda: {error}");
            }
        }
    }

    pub(super) fn push_venda_field(&mut self, character: char) {
        if self.venda_resumo_focus {
            return;
        }
        match self.venda_field {
            VendaField::Tecido | VendaField::Vinculo => {}
            VendaField::Preco => self.venda_preco.push(character),
            VendaField::Quantidade => self.venda_quantidade.push(character),
            VendaField::Finalizar | VendaField::Cancelar | VendaField::Excluir => {}
        }
    }

    pub(super) fn backspace_venda_field(&mut self) {
        if self.vendas_screen == VendasScreen::Historico {
            self.backspace_venda_historico_field();
            return;
        }
        if self.venda_resumo_focus {
            return;
        }
        match self.venda_field {
            VendaField::Tecido | VendaField::Vinculo => {}
            VendaField::Preco => {
                self.venda_preco.pop();
            }
            VendaField::Quantidade => {
                self.venda_quantidade.pop();
            }
            VendaField::Finalizar | VendaField::Cancelar | VendaField::Excluir => {}
        }
    }

    fn normalize_venda_field(&mut self) {
        if self.editing_venda_id.is_none() && self.venda_field == VendaField::Excluir {
            self.venda_field = VendaField::Tecido;
        }
    }

    fn next_venda_item(&mut self) {
        if !self.venda_itens.is_empty() {
            self.venda_item_option = (self.venda_item_option + 1) % self.venda_itens.len();
        }
    }

    fn previous_venda_item(&mut self) {
        if !self.venda_itens.is_empty() {
            self.venda_item_option =
                (self.venda_item_option + self.venda_itens.len() - 1) % self.venda_itens.len();
        }
    }

    fn editar_venda_item_selecionado(&mut self) {
        let Some(item) = self.venda_itens.get(self.venda_item_option) else {
            return;
        };
        self.editing_venda_item = Some(self.venda_item_option);
        self.editing_venda_item_descricao = Some(item.descricao.clone());
        self.venda_preco = format_number_input(item.preco_unitario);
        self.venda_quantidade = format_number_input(item.quantidade);
        self.venda_field = VendaField::Preco;
        self.venda_resumo_focus = false;
        self.db_status = format!("Editando lancamento {}", self.venda_item_option + 1);
    }

    fn delete_venda_item(&mut self) {
        if self.vendas_screen == VendasScreen::Lancamento
            && self.venda_resumo_focus
            && !self.venda_itens.is_empty()
        {
            self.pending_delete_venda_item = true;
        }
    }

    fn excluir_venda_item_confirmado(&mut self) {
        if self.venda_item_option < self.venda_itens.len() {
            let removed_index = self.venda_item_option;
            self.venda_itens.remove(removed_index);

            if let Some(venda_id) = self.editing_venda_id {
                let Some(pool) = &self.db_pool else {
                    self.db_status = String::from("Banco local indisponivel para salvar exclusao");
                    self.pending_delete_venda_item = false;
                    return;
                };
                if let Err(error) =
                    self.db_runtime
                        .block_on(db::update_venda(pool, venda_id, &self.venda_itens))
                {
                    self.db_status = format!("Erro ao salvar exclusao: {error}");
                    self.pending_delete_venda_item = false;
                    return;
                }
                self.reload_vendas_historico();
            }

            self.venda_item_option = self
                .venda_item_option
                .min(self.venda_itens.len().saturating_sub(1));
            if self.editing_venda_item == Some(removed_index) {
                self.reset_venda_item_editing();
            }
            self.venda_resumo_focus = !self.venda_itens.is_empty();
            self.db_status = if self.editing_venda_id.is_some() {
                format!("Lancamento {} excluido e salvo", removed_index + 1)
            } else {
                format!("Lancamento {} excluido", removed_index + 1)
            };
        }
        self.pending_delete_venda_item = false;
    }

    pub(super) fn reset_venda_item_editing(&mut self) {
        self.venda_resumo_focus = false;
        self.venda_item_option = 0;
        self.editing_venda_item = None;
        self.editing_venda_item_descricao = None;
        self.pending_delete_venda_item = false;
    }

    pub(super) fn voltar_vendas(&mut self) {
        if self.vendas_screen == VendasScreen::Lancamento && self.pending_delete_venda_item {
            self.pending_delete_venda_item = false;
            return;
        }

        if self.vendas_screen == VendasScreen::Lancamento && self.venda_resumo_focus {
            self.venda_resumo_focus = false;
            return;
        }

        if self.vendas_screen == VendasScreen::Lancamento && self.editing_venda_item.is_some() {
            self.editing_venda_item = None;
            self.editing_venda_item_descricao = None;
            self.venda_preco.clear();
            self.venda_quantidade.clear();
            self.db_status = String::from("Edicao do lancamento cancelada");
            return;
        }

        if self.editing_venda_id.is_some() && self.vendas_screen == VendasScreen::Lancamento {
            self.vendas_screen = VendasScreen::Historico;
            self.editing_venda_id = None;
            self.pending_delete_venda = false;
            self.finalizar_venda_dialog = false;
            self.venda_dropdown = None;
            self.reset_venda_item_editing();
            return;
        }

        self.vendas_screen = match self.vendas_screen {
            VendasScreen::Menu => VendasScreen::Menu,
            VendasScreen::SelecionarTecido | VendasScreen::Historico => VendasScreen::Menu,
            VendasScreen::SelecionarVinculo => VendasScreen::SelecionarTecido,
            VendasScreen::Lancamento => VendasScreen::SelecionarVinculo,
        };
        self.finalizar_venda_dialog = false;
        self.venda_dropdown = None;
        self.pending_delete_venda = false;
        self.pending_delete_venda_item = false;
        if self.vendas_screen != VendasScreen::Lancamento {
            self.editing_venda_id = None;
            self.reset_venda_item_editing();
        }
    }
}

fn format_number_input(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}").replace('.', ",")
    }
}
