use std::{
    fs, panic,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crossterm::event::KeyCode;

use super::{App, PedidoPdfResult, pdf_actions};
use crate::{db, models::*};

mod pdf;

impl App {
    pub(super) fn handle_pedidos_key(&mut self, key: KeyCode) {
        if self.finalizar_pedido_dialog {
            match key {
                KeyCode::Esc => self.finalizar_pedido_dialog = false,
                KeyCode::Left | KeyCode::Up => {
                    self.finalizar_pedido_option = self.finalizar_pedido_option.previous();
                }
                KeyCode::Right | KeyCode::Down => {
                    self.finalizar_pedido_option = self.finalizar_pedido_option.next();
                }
                KeyCode::Enter => self.confirmar_finalizacao_pedido(),
                _ => {}
            }
            return;
        }

        if self.pending_approve_pedido {
            match key {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.pending_approve_pedido = false;
                }
                KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.aprovar_pedido_confirmado();
                }
                _ => {}
            }
            return;
        }

        if self.pending_delete_pedido {
            match key {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.pending_delete_pedido = false;
                }
                KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.excluir_pedido_confirmado();
                }
                _ => {}
            }
            return;
        }

        if self.pedidos_screen == PedidosScreen::Lancamento && self.pedido_dropdown.is_some() {
            match key {
                KeyCode::Esc | KeyCode::Enter => self.pedido_dropdown = None,
                KeyCode::Up => self.previous_pedido_dropdown_option(),
                KeyCode::Down => self.next_pedido_dropdown_option(),
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Esc => self.voltar_pedidos(),
            KeyCode::Backspace => self.backspace_pedido_field(),
            KeyCode::Delete => self.delete_pedido_item(),
            KeyCode::Up => self.previous_pedido_option(),
            KeyCode::Down => self.next_pedido_option(),
            KeyCode::Left => self.section = self.section.previous(),
            KeyCode::Right => self.section = self.section.next(),
            KeyCode::Enter => self.enter_pedidos(),
            KeyCode::Char(character)
                if !character.is_control() && self.pedidos_screen == PedidosScreen::Lancamento =>
            {
                self.push_pedido_field(character);
            }
            _ => {}
        }
    }

    fn next_pedido_option(&mut self) {
        match self.pedidos_screen {
            PedidosScreen::Menu => self.pedido_menu_option = (self.pedido_menu_option + 1) % 2,
            PedidosScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.pedido_tecido_option =
                        (self.pedido_tecido_option + 1) % self.tecidos.len();
                }
            }
            PedidosScreen::SelecionarVinculo => {
                if !self.pedido_vinculos.is_empty() {
                    self.pedido_vinculo_option =
                        (self.pedido_vinculo_option + 1) % self.pedido_vinculos.len();
                }
            }
            PedidosScreen::Lancamento => {
                if self.pedido_resumo_focus {
                    self.next_pedido_item();
                } else {
                    self.pedido_field = self.next_pedido_field();
                    self.normalize_pedido_field();
                }
            }
            PedidosScreen::Historico => {
                if !self.pedidos_historico.is_empty() {
                    self.pedido_historico_option =
                        (self.pedido_historico_option + 1) % self.pedidos_historico.len();
                }
            }
        }
    }

    fn previous_pedido_option(&mut self) {
        match self.pedidos_screen {
            PedidosScreen::Menu => self.pedido_menu_option = (self.pedido_menu_option + 1) % 2,
            PedidosScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.pedido_tecido_option =
                        (self.pedido_tecido_option + self.tecidos.len() - 1) % self.tecidos.len();
                }
            }
            PedidosScreen::SelecionarVinculo => {
                if !self.pedido_vinculos.is_empty() {
                    self.pedido_vinculo_option =
                        (self.pedido_vinculo_option + self.pedido_vinculos.len() - 1)
                            % self.pedido_vinculos.len();
                }
            }
            PedidosScreen::Lancamento => {
                if self.pedido_resumo_focus {
                    self.previous_pedido_item();
                } else {
                    self.pedido_field = self.previous_pedido_field();
                    self.normalize_pedido_field();
                }
            }
            PedidosScreen::Historico => {
                if !self.pedidos_historico.is_empty() {
                    self.pedido_historico_option =
                        (self.pedido_historico_option + self.pedidos_historico.len() - 1)
                            % self.pedidos_historico.len();
                }
            }
        }
    }

    fn enter_pedidos(&mut self) {
        match self.pedidos_screen {
            PedidosScreen::Menu => {
                self.pedido_resumo_focus = false;
                if self.pedido_menu_option == 0 {
                    self.pedidos_screen = PedidosScreen::SelecionarTecido;
                    self.pedido_tecido_option = 0;
                    self.pedido_itens.clear();
                    self.editing_pedido_id = None;
                    self.editing_pedido_item = None;
                } else {
                    self.reload_pedidos_historico();
                    self.pedidos_screen = PedidosScreen::Historico;
                }
            }
            PedidosScreen::SelecionarTecido => self.open_pedido_vinculos(),
            PedidosScreen::SelecionarVinculo => {
                if !self.pedido_vinculos.is_empty() {
                    self.pedidos_screen = PedidosScreen::Lancamento;
                    self.pedido_field = VendaField::Preco;
                    self.pedido_dropdown = None;
                    self.pedido_preco_option = 0;
                    self.apply_pedido_preco_option();
                    self.pedido_quantidade.clear();
                }
            }
            PedidosScreen::Lancamento => {
                if self.pedido_resumo_focus {
                    self.editar_pedido_item_selecionado();
                    return;
                }
                if matches!(
                    self.pedido_field,
                    VendaField::Tecido | VendaField::Vinculo | VendaField::Preco
                ) {
                    self.pedido_dropdown = Some(self.pedido_field);
                    return;
                }
                match self.pedido_field {
                    VendaField::Quantidade => self.confirmar_lancamento_pedido(),
                    VendaField::Finalizar if self.editing_pedido_id.is_some() => {
                        self.pending_approve_pedido = true
                    }
                    VendaField::Finalizar => self.abrir_dialog_finalizar_pedido(),
                    VendaField::Cancelar if self.editing_pedido_id.is_some() => {
                        self.pending_delete_pedido = true
                    }
                    VendaField::Cancelar => self.cancelar_pedido(),
                    VendaField::Excluir if self.editing_pedido_id.is_some() => {
                        self.compartilhar_pedido_atual()
                    }
                    _ => self.pedido_field = self.pedido_field.next(),
                }
            }
            PedidosScreen::Historico => self.open_pedido_historico(),
        }
    }

    fn open_pedido_vinculos(&mut self) {
        let Some(tecido) = self.tecidos.get(self.pedido_tecido_option) else {
            self.pedido_vinculos.clear();
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
                Ok(vinculos) => self.pedido_vinculos = vinculos,
                Err(error) => self.db_status = format!("Erro ao carregar vinculos pedido: {error}"),
            }
        }
        self.pedido_vinculo_option = 0;
        self.pedidos_screen = PedidosScreen::SelecionarVinculo;
    }

    fn reload_pedido_vinculos_for_current_tecido(&mut self) {
        let current_screen = self.pedidos_screen;
        self.open_pedido_vinculos();
        self.pedidos_screen = current_screen;
    }

    fn next_pedido_dropdown_option(&mut self) {
        match self.pedido_dropdown {
            Some(VendaField::Tecido) if !self.tecidos.is_empty() => {
                self.pedido_tecido_option = (self.pedido_tecido_option + 1) % self.tecidos.len();
                self.reload_pedido_vinculos_for_current_tecido();
            }
            Some(VendaField::Vinculo) if !self.pedido_vinculos.is_empty() => {
                self.pedido_vinculo_option =
                    (self.pedido_vinculo_option + 1) % self.pedido_vinculos.len();
                self.apply_pedido_preco_option();
            }
            Some(VendaField::Preco) => self.next_pedido_preco_option(),
            _ => {}
        }
    }

    fn previous_pedido_dropdown_option(&mut self) {
        match self.pedido_dropdown {
            Some(VendaField::Tecido) if !self.tecidos.is_empty() => {
                self.pedido_tecido_option =
                    (self.pedido_tecido_option + self.tecidos.len() - 1) % self.tecidos.len();
                self.reload_pedido_vinculos_for_current_tecido();
            }
            Some(VendaField::Vinculo) if !self.pedido_vinculos.is_empty() => {
                self.pedido_vinculo_option =
                    (self.pedido_vinculo_option + self.pedido_vinculos.len() - 1)
                        % self.pedido_vinculos.len();
                self.apply_pedido_preco_option();
            }
            Some(VendaField::Preco) => self.previous_pedido_preco_option(),
            _ => {}
        }
    }

    fn confirmar_lancamento_pedido(&mut self) {
        if let Some(pedido_id) = self.editing_pedido_id
            && self.block_if_pedido_pdf_running_for(pedido_id, "alterar")
        {
            return;
        }
        let preco = parse_number(&self.pedido_preco).unwrap_or(0.0);
        let quantidade = parse_number(&self.pedido_quantidade).unwrap_or(0.0);
        if preco <= 0.0 || quantidade <= 0.0 {
            return;
        }
        let item = if let Some(index) = self.editing_pedido_item {
            let Some(current) = self.pedido_itens.get(index) else {
                return;
            };
            VendaItem {
                descricao: current.descricao.clone(),
                quantidade,
                preco_unitario: preco,
                estoque_tecido_id: current.estoque_tecido_id,
                estoque_item_id: current.estoque_item_id,
                estoque_usa_estampas: current.estoque_usa_estampas,
            }
        } else {
            let Some(vinculo) = self.pedido_vinculos.get(self.pedido_vinculo_option) else {
                return;
            };
            VendaItem {
                descricao: format!("{} - {}", vinculo.tecido_nome, vinculo.cor_nome),
                quantidade,
                preco_unitario: preco,
                estoque_tecido_id: self
                    .tecidos
                    .get(self.pedido_tecido_option)
                    .map(|tecido| tecido.id),
                estoque_item_id: Some(vinculo.cor_id),
                estoque_usa_estampas: self
                    .tecidos
                    .get(self.pedido_tecido_option)
                    .map(|tecido| tecido.tipo == "Estampado")
                    .unwrap_or(false),
            }
        };
        let editing_index = self.editing_pedido_item;
        if let Some(index) = editing_index {
            if let Some(current) = self.pedido_itens.get_mut(index) {
                *current = item;
                self.pedido_item_option = index;
            }
        } else {
            self.pedido_itens.push(item);
            self.pedido_item_option = self.pedido_itens.len().saturating_sub(1);
        }
        if let Some(pedido_id) = self.editing_pedido_id {
            let Some(pool) = &self.db_pool else {
                self.db_status = String::from("Banco local indisponivel para salvar pedido");
                return;
            };
            if let Err(error) = self.db_runtime.block_on(db::update_pedido_itens(
                pool,
                pedido_id,
                &self.pedido_itens,
            )) {
                self.db_status = format!("Erro ao salvar pedido: {error}");
                return;
            }
            self.reload_pedidos_historico();
        }
        self.pedido_preco_option = 0;
        self.apply_pedido_preco_option();
        self.pedido_quantidade.clear();
        self.pedido_dropdown = None;
        self.editing_pedido_item = None;
        self.db_status = if editing_index.is_some() {
            String::from("Lancamento do pedido atualizado")
        } else {
            String::from("Lancamento adicionado ao pedido")
        };
    }

    fn abrir_dialog_finalizar_pedido(&mut self) {
        if self.pedido_itens.is_empty() {
            self.db_status = String::from("Adicione ao menos um item antes de gerar pedido");
            return;
        }
        self.finalizar_pedido_dialog = true;
        self.finalizar_pedido_option = FinalizarVendaOption::Finalizar;
    }

    fn confirmar_finalizacao_pedido(&mut self) {
        let compartilhar = self.finalizar_pedido_option == FinalizarVendaOption::FinalizarEImprimir;
        self.gerar_pedido(compartilhar);
    }

    fn gerar_pedido(&mut self, compartilhar: bool) {
        if self.pedido_pdf_task.is_running() {
            self.db_status = String::from("Aguarde o PDF do pedido atual terminar.");
            return;
        }
        if self.pedido_itens.is_empty() {
            self.db_status = String::from("Adicione ao menos um item antes de gerar pedido");
            return;
        }
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para gerar pedido");
            self.finalizar_pedido_dialog = false;
            return;
        };
        let pedido_id =
            match self
                .db_runtime
                .block_on(db::insert_pedido(pool, &self.pedido_itens, None))
            {
                Ok(id) => id,
                Err(error) => {
                    self.db_status = format!("Erro ao salvar pedido: {error}");
                    return;
                }
            };
        self.start_pedido_pdf_worker(
            pedido_id,
            compartilhar,
            pool.clone(),
            String::from("Pedido salvo. Gerando PDF em segundo plano..."),
        );
        self.reload_pedidos_historico();
        self.pedido_itens.clear();
        self.pedido_vinculos.clear();
        self.pedido_preco_option = 0;
        self.pedido_preco.clear();
        self.pedido_quantidade.clear();
        self.finalizar_pedido_dialog = false;
        self.pedidos_screen = PedidosScreen::Menu;
    }

    fn open_pedido_historico(&mut self) {
        let Some(pedido) = self.pedidos_historico.get(self.pedido_historico_option) else {
            return;
        };
        let pedido_id = pedido.id;
        if self.block_if_pedido_pdf_running_for(pedido_id, "abrir") {
            return;
        }
        if let Some(pool) = &self.db_pool {
            match self
                .db_runtime
                .block_on(db::list_pedido_itens(pool, pedido_id))
            {
                Ok(itens) => {
                    self.pedido_itens = itens;
                    self.editing_pedido_id = Some(pedido_id);
                    self.editing_pedido_item = None;
                    self.pedidos_screen = PedidosScreen::Lancamento;
                    self.pedido_field = VendaField::Finalizar;
                    self.db_status = format!("Pedido #{pedido_id} aberto");
                }
                Err(error) => self.db_status = format!("Erro ao abrir pedido: {error}"),
            }
        }
    }

    fn compartilhar_pedido_atual(&mut self) {
        if self.pedido_pdf_task.is_running() {
            self.db_status = String::from("Aguarde o PDF do pedido atual terminar.");
            return;
        }
        let Some(pedido_id) = self.editing_pedido_id else {
            return;
        };
        let existing_pdf = self
            .pedidos_historico
            .iter()
            .find(|pedido| pedido.id == pedido_id)
            .and_then(|pedido| pedido.pdf_path.clone());

        let path_text = if let Some(path) = existing_pdf.filter(|path| Path::new(path).is_file()) {
            path
        } else {
            let Some(pool) = &self.db_pool else {
                self.db_status = String::from("Banco local indisponivel para compartilhar pedido");
                return;
            };
            self.start_pedido_pdf_worker(
                pedido_id,
                true,
                pool.clone(),
                String::from("Gerando PDF do pedido em segundo plano..."),
            );
            return;
        };

        self.db_status = match self.abrir_compartilhamento_pedido(pedido_id, &path_text) {
            Ok(()) => format!("Compartilhamento do pedido #{pedido_id} aberto: {path_text}"),
            Err(error) => format!("{error} PDF salvo em: {path_text}"),
        };
    }

    fn aprovar_pedido_confirmado(&mut self) {
        let Some(pedido_id) = self.editing_pedido_id else {
            self.pending_approve_pedido = false;
            return;
        };
        if self.block_if_pedido_pdf_running_for(pedido_id, "aprovar") {
            self.pending_approve_pedido = false;
            return;
        }
        if let Some(pool) = &self.db_pool {
            match self
                .db_runtime
                .block_on(db::approve_pedido(pool, pedido_id))
            {
                Ok(()) => {
                    self.reload_pedidos_historico();
                    self.reload_vendas_historico();
                    self.reload_estoque_saldos();
                    self.reload_estoque_ordens();
                    self.pedido_itens.clear();
                    self.editing_pedido_id = None;
                    self.editing_pedido_item = None;
                    self.pending_approve_pedido = false;
                    self.pedidos_screen = PedidosScreen::Historico;
                    self.db_status = format!("Pedido #{pedido_id} aprovado e convertido em venda");
                }
                Err(error) => self.db_status = format!("Erro ao aprovar pedido: {error}"),
            }
        }
    }

    fn excluir_pedido_confirmado(&mut self) {
        let Some(pedido_id) = self.editing_pedido_id else {
            self.pending_delete_pedido = false;
            return;
        };
        if self.block_if_pedido_pdf_running_for(pedido_id, "cancelar") {
            self.pending_delete_pedido = false;
            return;
        }
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para cancelar pedido");
            self.pending_delete_pedido = false;
            return;
        };
        match self.db_runtime.block_on(db::delete_pedido(pool, pedido_id)) {
            Ok(()) => {
                self.reload_pedidos_historico();
                self.pedido_itens.clear();
                self.pedido_vinculos.clear();
                self.pedido_preco.clear();
                self.pedido_quantidade.clear();
                self.editing_pedido_id = None;
                self.editing_pedido_item = None;
                self.pending_delete_pedido = false;
                self.pending_approve_pedido = false;
                self.pedido_resumo_focus = false;
                self.pedidos_screen = PedidosScreen::Historico;
                self.pedido_historico_option = self
                    .pedido_historico_option
                    .min(self.pedidos_historico.len().saturating_sub(1));
                self.db_status = format!("Pedido #{pedido_id} cancelado e removido do historico");
            }
            Err(error) => {
                self.pending_delete_pedido = false;
                self.db_status = format!("Erro ao cancelar pedido: {error}");
            }
        }
    }

    fn reload_pedidos_historico(&mut self) {
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_pedidos(pool)) {
                Ok(pedidos) => self.pedidos_historico = pedidos,
                Err(error) => self.db_status = format!("Erro ao carregar pedidos: {error}"),
            }
        }
    }

    fn cancelar_pedido(&mut self) {
        self.pedido_itens.clear();
        self.pedido_vinculos.clear();
        self.pedido_preco_option = 0;
        self.pedido_preco.clear();
        self.pedido_quantidade.clear();
        self.editing_pedido_id = None;
        self.editing_pedido_item = None;
        self.pending_approve_pedido = false;
        self.pending_delete_pedido = false;
        self.finalizar_pedido_dialog = false;
        self.pedido_resumo_focus = false;
        self.pedidos_screen = PedidosScreen::Menu;
        self.db_status = String::from("Pedido cancelado");
    }

    fn push_pedido_field(&mut self, character: char) {
        if self.pedido_resumo_focus {
            return;
        }
        match self.pedido_field {
            VendaField::Preco => {
                if self.pedido_preco_option != 2 {
                    self.pedido_preco_option = 2;
                    self.pedido_preco.clear();
                }
                self.pedido_preco.push(character);
            }
            VendaField::Quantidade => self.pedido_quantidade.push(character),
            _ => {}
        }
    }

    fn backspace_pedido_field(&mut self) {
        match self.pedido_field {
            VendaField::Preco => {
                if self.pedido_preco_option != 2 {
                    self.pedido_preco_option = 2;
                    self.pedido_preco.clear();
                } else {
                    self.pedido_preco.pop();
                }
            }
            VendaField::Quantidade => {
                self.pedido_quantidade.pop();
            }
            _ => {}
        }
    }

    fn delete_pedido_item(&mut self) {
        if self.pedidos_screen == PedidosScreen::Lancamento
            && self.pedido_resumo_focus
            && self.pedido_item_option < self.pedido_itens.len()
        {
            if let Some(pedido_id) = self.editing_pedido_id
                && self.block_if_pedido_pdf_running_for(pedido_id, "alterar")
            {
                return;
            }
            let removed_index = self.pedido_item_option;
            self.pedido_itens.remove(self.pedido_item_option);
            if let Some(pedido_id) = self.editing_pedido_id {
                let Some(pool) = &self.db_pool else {
                    self.db_status = String::from("Banco local indisponivel para salvar exclusao");
                    return;
                };
                if let Err(error) = self.db_runtime.block_on(db::update_pedido_itens(
                    pool,
                    pedido_id,
                    &self.pedido_itens,
                )) {
                    self.db_status = format!("Erro ao salvar exclusao do pedido: {error}");
                    return;
                }
                self.reload_pedidos_historico();
            }
            self.pedido_item_option = self
                .pedido_item_option
                .min(self.pedido_itens.len().saturating_sub(1));
            if self.editing_pedido_item == Some(removed_index) {
                self.editing_pedido_item = None;
                self.pedido_preco.clear();
                self.pedido_quantidade.clear();
            }
            self.pedido_resumo_focus = !self.pedido_itens.is_empty();
            self.db_status = if self.editing_pedido_id.is_some() {
                format!("Item {} removido do pedido e salvo", removed_index + 1)
            } else {
                format!("Item {} removido do pedido", removed_index + 1)
            };
        }
    }

    fn next_pedido_item(&mut self) {
        if !self.pedido_itens.is_empty() {
            self.pedido_item_option = (self.pedido_item_option + 1) % self.pedido_itens.len();
        }
    }

    fn previous_pedido_item(&mut self) {
        if !self.pedido_itens.is_empty() {
            self.pedido_item_option =
                (self.pedido_item_option + self.pedido_itens.len() - 1) % self.pedido_itens.len();
        }
    }

    fn editar_pedido_item_selecionado(&mut self) {
        let Some(item) = self.pedido_itens.get(self.pedido_item_option) else {
            return;
        };
        self.editing_pedido_item = Some(self.pedido_item_option);
        self.pedido_preco_option = 2;
        self.pedido_preco = format_number_input(item.preco_unitario);
        self.pedido_quantidade = format_number_input(item.quantidade);
        self.pedido_field = VendaField::Preco;
        self.pedido_resumo_focus = false;
        self.db_status = format!("Editando lancamento {}", self.pedido_item_option + 1);
    }

    fn normalize_pedido_field(&mut self) {
        if self.editing_pedido_id.is_none() && self.pedido_field == VendaField::Excluir {
            self.pedido_field = VendaField::Tecido;
        }
    }

    fn next_pedido_field(&self) -> VendaField {
        next_in_order(
            self.pedido_field,
            pedido_field_order(self.editing_pedido_id.is_some()),
        )
    }

    fn previous_pedido_field(&self) -> VendaField {
        previous_in_order(
            self.pedido_field,
            pedido_field_order(self.editing_pedido_id.is_some()),
        )
    }

    fn next_pedido_preco_option(&mut self) {
        self.set_pedido_preco_option((self.pedido_preco_option + 1) % 3);
    }

    fn previous_pedido_preco_option(&mut self) {
        self.set_pedido_preco_option((self.pedido_preco_option + 2) % 3);
    }

    fn set_pedido_preco_option(&mut self, option: usize) {
        self.pedido_preco_option = option;
        if self.pedido_preco_option == 2 {
            self.pedido_preco.clear();
            return;
        }
        self.apply_pedido_preco_option();
    }

    fn apply_pedido_preco_option(&mut self) {
        if self.pedido_preco_option == 2 {
            return;
        }
        self.pedido_preco = self
            .pedido_vinculos
            .get(self.pedido_vinculo_option)
            .and_then(|vinculo| preco_option_value(vinculo, self.pedido_preco_option))
            .map(format_number_input)
            .unwrap_or_default();
    }

    fn voltar_pedidos(&mut self) {
        if self.pedidos_screen == PedidosScreen::Lancamento && self.pedido_resumo_focus {
            self.pedido_resumo_focus = false;
            return;
        }
        if self.pedidos_screen == PedidosScreen::Lancamento && self.editing_pedido_item.is_some() {
            self.editing_pedido_item = None;
            self.pedido_preco_option = 0;
            self.pedido_preco.clear();
            self.pedido_quantidade.clear();
            self.db_status = String::from("Edicao do lancamento cancelada");
            return;
        }
        self.pedidos_screen = match self.pedidos_screen {
            PedidosScreen::Menu => PedidosScreen::Menu,
            PedidosScreen::SelecionarTecido | PedidosScreen::Historico => PedidosScreen::Menu,
            PedidosScreen::SelecionarVinculo => PedidosScreen::SelecionarTecido,
            PedidosScreen::Lancamento => PedidosScreen::Menu,
        };
        self.pending_approve_pedido = false;
        self.pending_delete_pedido = false;
        self.finalizar_pedido_dialog = false;
        self.pedido_dropdown = None;
        self.pedido_preco_option = 0;
        if self.pedidos_screen != PedidosScreen::Lancamento {
            self.editing_pedido_id = None;
            self.editing_pedido_item = None;
        }
    }

    pub(super) fn abrir_compartilhamento_pedido(
        &self,
        pedido_id: i64,
        path: &str,
    ) -> Result<(), String> {
        abrir_compartilhamento_pedido(pedido_id, path)
    }

    fn start_pedido_pdf_worker(
        &mut self,
        pedido_id: i64,
        compartilhar: bool,
        pool: sqlx::PgPool,
        status: String,
    ) {
        self.db_status = status;
        self.pedido_pdf_task_id = Some(pedido_id);
        self.pedido_pdf_task
            .start(move || gerar_pdf_pedido_worker(pool, pedido_id, compartilhar));
    }

    fn block_if_pedido_pdf_running_for(&mut self, pedido_id: i64, action: &str) -> bool {
        if self.pedido_pdf_task_id == Some(pedido_id) && self.pedido_pdf_task.is_running() {
            self.db_status =
                format!("Aguarde o PDF do pedido #{pedido_id} terminar antes de {action}.");
            true
        } else {
            false
        }
    }
}

fn gerar_pdf_pedido_worker(
    pool: sqlx::PgPool,
    pedido_id: i64,
    compartilhar: bool,
) -> PedidoPdfResult {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            return PedidoPdfResult {
                pedido_id,
                compartilhar: false,
                pdf_path: None,
                status: format!("Pedido #{pedido_id} salvo. Erro ao iniciar PDF: {error}"),
            };
        }
    };
    let itens = match runtime.block_on(db::list_pedido_itens(&pool, pedido_id)) {
        Ok(itens) => itens,
        Err(error) => {
            return PedidoPdfResult {
                pedido_id,
                compartilhar: false,
                pdf_path: None,
                status: format!(
                    "Pedido #{pedido_id} salvo. Erro ao carregar itens do PDF: {error}"
                ),
            };
        }
    };
    let path = match gerar_pdf_pedido_file(pedido_id, &itens) {
        Ok(path) => path,
        Err(error) => {
            return PedidoPdfResult {
                pedido_id,
                compartilhar: false,
                pdf_path: None,
                status: format!("Pedido #{pedido_id} salvo, erro ao gerar PDF: {error}"),
            };
        }
    };
    let path_text = path.to_string_lossy().to_string();
    let update_status =
        match runtime.block_on(db::update_pedido_pdf_path(&pool, pedido_id, &path_text)) {
            Ok(()) => None,
            Err(error) => Some(format!(" Falha ao salvar caminho no banco: {error}")),
        }
        .unwrap_or_default();

    PedidoPdfResult {
        pedido_id,
        compartilhar,
        pdf_path: Some(path_text.clone()),
        status: format!("Pedido #{pedido_id} gerado. PDF: {path_text}{update_status}"),
    }
}

fn gerar_pdf_pedido_file(pedido_id: i64, itens: &[VendaItem]) -> Result<PathBuf, String> {
    let dir = pedidos_pdf_dir()?;
    let path = dir.join(format!("razai_pedido_{pedido_id}.pdf"));
    match write_pedido_pdf_catching(&path, pedido_id, itens) {
        Ok(()) => Ok(path),
        Err(error) if should_retry_pedido_pdf_with_unique_path(&error) => {
            let fallback = dir.join(unique_pedido_pdf_name(pedido_id));
            write_pedido_pdf_catching(&fallback, pedido_id, itens).map(|()| fallback)
        }
        Err(error) => Err(error),
    }
}

fn write_pedido_pdf_catching(
    path: &Path,
    pedido_id: i64,
    itens: &[VendaItem],
) -> Result<(), String> {
    panic::catch_unwind(|| pdf::write_pedido_pdf(path, pedido_id, itens)).map_err(|_| {
        String::from("falha interna ao gerar PDF do pedido; revise textos dos itens")
    })?
}

fn should_retry_pedido_pdf_with_unique_path(error: &str) -> bool {
    error.contains("os error 1224")
        || error.contains("seção mapeada")
        || error.contains("secao mapeada")
        || error.contains("mapped section")
}

fn unique_pedido_pdf_name(pedido_id: i64) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!(
        "razai_pedido_{pedido_id}_{}.pdf",
        millis + u128::from(std::process::id())
    )
}

fn abrir_compartilhamento_pedido(pedido_id: i64, path: &str) -> Result<(), String> {
    let file_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("pedido.pdf");
    pdf_actions::share_pdf_with_windows_ui(
        &format!("Pedido #{pedido_id}"),
        Path::new(path),
        &format!("Pedido #{pedido_id} Razai"),
        "PDF do pedido gerado pelo Razai.",
        &format!("Pedido Razai: {file_name}"),
    )
}

fn pedidos_pdf_dir() -> Result<PathBuf, String> {
    let dir = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|home| home.join("Documents").join("Razai").join("pedidos"))
        .unwrap_or_else(|| std::env::temp_dir().join("Razai").join("pedidos"));
    fs::create_dir_all(&dir)
        .map_err(|error| format!("falha ao criar pasta de PDFs do pedido: {error}"))?;
    Ok(dir)
}

fn pedido_field_order(editing: bool) -> &'static [VendaField] {
    if editing {
        &[
            VendaField::Tecido,
            VendaField::Vinculo,
            VendaField::Preco,
            VendaField::Quantidade,
            VendaField::Finalizar,
            VendaField::Excluir,
            VendaField::Cancelar,
        ]
    } else {
        &[
            VendaField::Tecido,
            VendaField::Vinculo,
            VendaField::Preco,
            VendaField::Quantidade,
            VendaField::Finalizar,
            VendaField::Cancelar,
        ]
    }
}

fn next_in_order(current: VendaField, order: &[VendaField]) -> VendaField {
    let index = order
        .iter()
        .position(|field| *field == current)
        .unwrap_or(0);
    order[(index + 1) % order.len()]
}

fn previous_in_order(current: VendaField, order: &[VendaField]) -> VendaField {
    let index = order
        .iter()
        .position(|field| *field == current)
        .unwrap_or(0);
    order[(index + order.len() - 1) % order.len()]
}

fn preco_option_value(vinculo: &db::VinculoRecord, option: usize) -> Option<f64> {
    match option {
        0 => vinculo.preco_atacado_efetivo,
        1 => vinculo.preco_varejo_efetivo,
        _ => None,
    }
}

fn format_number_input(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}").replace('.', ",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedido_pdf_dir_stays_outside_workspace() {
        let dir = pedidos_pdf_dir().unwrap();
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        assert!(
            !dir.starts_with(&workspace),
            "pedido PDF dir must stay outside workspace to avoid cargo watch restarts: {}",
            dir.display()
        );
        assert!(dir.ends_with(Path::new("Razai").join("pedidos")));
    }

    #[test]
    fn pedido_pdf_file_is_generated_outside_workspace() {
        let pedido_id = -i64::from(std::process::id());
        let itens = vec![VendaItem {
            descricao: String::from("Tecido teste - Cor teste"),
            quantidade: 2.0,
            preco_unitario: 12.5,
            estoque_tecido_id: None,
            estoque_item_id: None,
            estoque_usa_estampas: false,
        }];

        let path = gerar_pdf_pedido_file(pedido_id, &itens).unwrap();
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        assert!(
            path.is_file(),
            "PDF should be generated at {}",
            path.display()
        );
        assert!(
            !path.starts_with(&workspace),
            "pedido PDF file must stay outside workspace: {}",
            path.display()
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn pedido_pdf_retries_with_unique_path_when_windows_keeps_previous_file_mapped() {
        assert!(should_retry_pedido_pdf_with_unique_path(
            "falha ao criar PDF: A operação solicitada não pode ser executada em um arquivo com uma seção mapeada pelo usuário aberta. (os error 1224)"
        ));
        assert!(!should_retry_pedido_pdf_with_unique_path(
            "falha ao criar PDF: acesso negado"
        ));
        assert!(unique_pedido_pdf_name(7).starts_with("razai_pedido_7_"));
    }

    #[test]
    fn pedido_historico_field_order_matches_visual_actions() {
        let order = pedido_field_order(true);

        assert!(
            order
                == &[
                    VendaField::Tecido,
                    VendaField::Vinculo,
                    VendaField::Preco,
                    VendaField::Quantidade,
                    VendaField::Finalizar,
                    VendaField::Excluir,
                    VendaField::Cancelar,
                ]
        );
        assert!(next_in_order(VendaField::Finalizar, order) == VendaField::Excluir);
        assert!(previous_in_order(VendaField::Cancelar, order) == VendaField::Excluir);
    }

    #[test]
    fn pedido_novo_field_order_omits_compartilhar() {
        let order = pedido_field_order(false);

        assert!(!order.contains(&VendaField::Excluir));
        assert!(next_in_order(VendaField::Finalizar, order) == VendaField::Cancelar);
    }
}
