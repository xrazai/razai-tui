use std::{
    fs,
    path::Path,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use super::{App, VinculoImageUploadResult};
use crate::{
    db::{self, TecidoRecord, VinculoImages, VinculoRecord},
    models::*,
};
use crossterm::event::KeyCode;
use image::{DynamicImage, ImageError, ImageReader, imageops::FilterType};
use ratatui::layout::Size;
use ratatui_image::Resize;
use sqlx::PgPool;

mod navigation;

impl App {
    pub(super) fn handle_tecido_form_key(&mut self, key: KeyCode) {
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

        if self.tecido_select_dropdown.is_some() {
            match key {
                KeyCode::Esc | KeyCode::Enter => self.tecido_select_dropdown = None,
                KeyCode::Up => self.tecido_form.previous_select_option(),
                KeyCode::Down => self.tecido_form.next_select_option(),
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Esc => self.voltar_dados(),
            KeyCode::Backspace => self.tecido_form.backspace(),
            KeyCode::Up => self.tecido_form.previous_field(),
            KeyCode::Down => self.tecido_form.next_field(),
            KeyCode::Enter if self.tecido_form.selected_field == TecidoField::Salvar => {
                self.cadastrar_tecido();
            }
            KeyCode::Enter if self.tecido_form.selected_field == TecidoField::Excluir => {
                self.pending_delete = true;
            }
            KeyCode::Enter if self.tecido_form.selected_field == TecidoField::Voltar => {
                self.voltar_dados();
            }
            KeyCode::Enter if self.tecido_form.selected_field.is_select() => {
                self.tecido_select_dropdown = Some(self.tecido_form.selected_field);
            }
            KeyCode::Enter => self.tecido_form.next_field(),
            KeyCode::Char(character) => self.tecido_form.push(character),
            _ => {}
        }
    }

    pub(super) fn handle_cor_form_key(&mut self, key: KeyCode) {
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
            KeyCode::Down => self.cor_form.next_field(),
            KeyCode::Enter if self.cor_form.selected_field == CorField::Confirmar => {
                self.confirmar_cor();
            }
            KeyCode::Enter if self.cor_form.selected_field == CorField::Voltar => {
                self.voltar_dados();
            }
            KeyCode::Enter if self.cor_form.selected_field == CorField::Excluir => {
                self.pending_delete = true;
            }
            KeyCode::Enter => self.cor_form.next_field(),
            KeyCode::Char(character) => self.cor_form.push(character),
            _ => {}
        }
    }

    pub(super) fn handle_estampa_form_key(&mut self, key: KeyCode) {
        if self.pending_delete {
            match key {
                KeyCode::Char('s') | KeyCode::Char('S') => self.excluir_estampa_confirmada(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.pending_delete = false
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Esc => self.voltar_dados(),
            KeyCode::Backspace => self.estampa_form.backspace(),
            KeyCode::Up => self.estampa_form.previous_field(),
            KeyCode::Down => self.estampa_form.next_field(),
            KeyCode::Enter if self.estampa_form.selected_field == EstampaField::Confirmar => {
                self.confirmar_estampa();
            }
            KeyCode::Enter if self.estampa_form.selected_field == EstampaField::Voltar => {
                self.voltar_dados();
            }
            KeyCode::Enter if self.estampa_form.selected_field == EstampaField::Excluir => {
                self.pending_delete = true;
            }
            KeyCode::Enter => self.estampa_form.next_field(),
            KeyCode::Char(character) => self.estampa_form.push(character),
            _ => {}
        }
    }
    pub(super) fn voltar_dados(&mut self) {
        match self.dados_screen {
            DadosScreen::CadastrarTecido => {
                self.tecido_form = TecidoForm::default();
                self.tecido_select_dropdown = None;
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
            DadosScreen::CadastrarEstampa => {
                self.estampa_form = EstampaForm::default();
                self.editing_estampa_id = None;
                self.pending_delete = false;
                self.dados_screen = DadosScreen::Estampas;
            }
            DadosScreen::VinculosSelecionarCores | DadosScreen::VinculosLista => {
                self.dados_screen = DadosScreen::VinculosMenu;
            }
            DadosScreen::VinculoDetalhe => {
                self.pending_unlink_vinculo = false;
                self.dados_screen = DadosScreen::VinculosLista;
            }
            DadosScreen::VinculosSelecionarTecidoCriar
            | DadosScreen::VinculosSelecionarTecidoVer => {
                self.dados_screen = DadosScreen::VinculosMenu;
            }
            DadosScreen::Tecidos
            | DadosScreen::Cores
            | DadosScreen::Estampas
            | DadosScreen::VinculosMenu => {
                self.dados_screen = DadosScreen::Menu;
            }
            DadosScreen::Menu => {
                self.section = Section::Dashboard;
            }
        }
    }

    pub(super) fn cadastrar_tecido(&mut self) {
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
        self.tecido_select_dropdown = None;
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

    pub(super) fn open_new_tecido(&mut self) {
        self.tecido_form = TecidoForm::default();
        self.tecido_select_dropdown = None;
        self.editing_tecido_id = None;
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarTecido;
    }

    pub(super) fn open_edit_tecido(&mut self, index: usize) {
        let Some(tecido) = self.tecidos.get(index) else {
            return;
        };
        self.tecido_form = TecidoForm::from_record(tecido);
        self.tecido_select_dropdown = None;
        self.editing_tecido_id = Some(tecido.id);
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarTecido;
    }

    pub(super) fn excluir_tecido_confirmado(&mut self) {
        let Some(id) = self.editing_tecido_id else {
            self.pending_delete = false;
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para excluir tecido");
            self.pending_delete = false;
            return;
        };

        if let Err(error) = self.db_runtime.block_on(db::delete_tecido(pool, id)) {
            self.db_status = format!("Erro ao excluir no banco: {error}");
            self.pending_delete = false;
            return;
        }

        self.reload_tecidos();
        self.db_status = String::from("Tecido excluido do banco local");
        self.tecido_form = TecidoForm::default();
        self.tecido_select_dropdown = None;
        self.editing_tecido_id = None;
        self.pending_delete = false;
        self.tecido_option = 0;
        self.dados_screen = DadosScreen::Tecidos;
    }

    pub(super) fn open_new_cor(&mut self) {
        self.cor_form = CorForm::default();
        self.editing_cor_id = None;
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarCor;
    }

    pub(super) fn open_edit_cor(&mut self, index: usize) {
        let Some(cor) = self.cores.get(index) else {
            return;
        };
        self.cor_form = CorForm::from_record(cor);
        self.editing_cor_id = Some(cor.id);
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarCor;
    }

    pub(super) fn confirmar_cor(&mut self) {
        if !self.cor_form.is_valid() {
            return;
        }
        let nearby = nearby_colors(
            &self.cor_form.hex,
            &self.cores,
            self.editing_cor_id,
            self.color_delta_e_threshold,
        );
        if let Some(color) = nearby.first() {
            self.db_status = format!(
                "Cor bloqueada: Delta E {:.2} para {} ({}) abaixo do limiar {:.2}",
                color.delta_e,
                color.nome,
                color.sku.as_deref().unwrap_or("sem SKU"),
                self.color_delta_e_threshold
            );
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

    pub(super) fn excluir_cor_confirmada(&mut self) {
        let Some(id) = self.editing_cor_id else {
            self.pending_delete = false;
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para excluir cor");
            self.pending_delete = false;
            return;
        };

        if let Err(error) = self.db_runtime.block_on(db::delete_cor(pool, id)) {
            self.db_status = format!("Erro ao excluir cor: {error}");
            self.pending_delete = false;
            return;
        }
        self.reload_cores();
        self.db_status = String::from("Cor excluida do banco local");
        self.cor_form = CorForm::default();
        self.editing_cor_id = None;
        self.pending_delete = false;
        self.cor_option = 0;
        self.dados_screen = DadosScreen::Cores;
    }

    pub(super) fn open_new_estampa(&mut self) {
        self.estampa_form = EstampaForm::default();
        self.editing_estampa_id = None;
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarEstampa;
    }

    pub(super) fn open_edit_estampa(&mut self, index: usize) {
        let Some(estampa) = self.estampas.get(index) else {
            return;
        };
        self.estampa_form = EstampaForm::from_record(estampa);
        self.editing_estampa_id = Some(estampa.id);
        self.pending_delete = false;
        self.dados_screen = DadosScreen::CadastrarEstampa;
    }

    pub(super) fn confirmar_estampa(&mut self) {
        if !self.estampa_form.is_valid() {
            return;
        }

        let nome = self.estampa_form.nome.trim().to_string();
        if let Some(pool) = &self.db_pool {
            let sku = self
                .estampa_form
                .sku(&self.estampas, self.editing_estampa_id);
            let result = match self.editing_estampa_id {
                Some(id) => self.db_runtime.block_on(db::update_estampa(
                    pool,
                    id,
                    &self.estampa_form.nome,
                    &sku,
                )),
                None => self.db_runtime.block_on(db::insert_estampa(
                    pool,
                    &self.estampa_form.nome,
                    &sku,
                )),
            };
            if let Err(error) = result {
                self.db_status = format!("Erro ao salvar estampa: {error}");
                return;
            }
        }

        self.reload_estampas();
        self.db_status = String::from("Estampa salva no banco local");
        self.estampa_form = EstampaForm::default();
        self.editing_estampa_id = None;
        self.pending_delete = false;
        self.cor_option = self
            .estampas
            .iter()
            .position(|estampa| estampa.nome == nome)
            .map(|index| index + 1)
            .unwrap_or(0);
        self.dados_screen = DadosScreen::Estampas;
    }

    pub(super) fn excluir_estampa_confirmada(&mut self) {
        let Some(id) = self.editing_estampa_id else {
            self.pending_delete = false;
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para excluir estampa");
            self.pending_delete = false;
            return;
        };

        if let Err(error) = self.db_runtime.block_on(db::delete_estampa(pool, id)) {
            self.db_status = format!("Erro ao excluir estampa: {error}");
            self.pending_delete = false;
            return;
        }
        self.reload_estampas();
        self.db_status = String::from("Estampa excluida do banco local");
        self.estampa_form = EstampaForm::default();
        self.editing_estampa_id = None;
        self.pending_delete = false;
        self.cor_option = 0;
        self.dados_screen = DadosScreen::Estampas;
    }

    pub(super) fn reload_tecidos(&mut self) {
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_tecidos(pool)) {
                Ok(tecidos) => self.tecidos = tecidos,
                Err(error) => self.db_status = format!("Erro ao recarregar tecidos: {error}"),
            }
        }
    }

    pub(super) fn reload_cores(&mut self) {
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_cores(pool)) {
                Ok(cores) => self.cores = cores,
                Err(error) => self.db_status = format!("Erro ao recarregar cores: {error}"),
            }
        }
    }

    pub(super) fn reload_estampas(&mut self) {
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_estampas(pool)) {
                Ok(estampas) => self.estampas = estampas,
                Err(error) => self.db_status = format!("Erro ao recarregar estampas: {error}"),
            }
        }
    }

    pub(super) fn selected_vinculo_tecido(&self) -> Option<&TecidoRecord> {
        self.tecidos.get(self.vinculo_tecido_option)
    }

    pub(super) fn open_vinculo_cores(&mut self) {
        let Some((tecido_id, usa_estampas)) = self
            .selected_vinculo_tecido()
            .map(|tecido| (tecido.id, tecido.tipo == "Estampado"))
        else {
            return;
        };
        self.load_vinculos(tecido_id);
        self.selected_vinculo_cores = self.vinculos.iter().map(|vinculo| vinculo.cor_id).collect();
        self.vinculo_criar_option = 0;
        self.dados_screen = DadosScreen::VinculosSelecionarCores;
        self.db_status = if usa_estampas {
            String::from("Vinculos de estampa para tecido estampado")
        } else {
            String::from("Vinculos de cor para tecido liso")
        };
    }

    pub(super) fn open_vinculo_lista(&mut self) {
        let Some(tecido_id) = self.selected_vinculo_tecido().map(|tecido| tecido.id) else {
            return;
        };
        self.load_vinculos(tecido_id);
        self.vinculo_lista_option = 0;
        self.dados_screen = DadosScreen::VinculosLista;
    }

    pub(super) fn open_vinculo_detalhe(&mut self) {
        if self.vinculos.is_empty() {
            return;
        }
        self.focus = Focus::System;
        self.pending_unlink_vinculo = false;
        self.editing_vinculo_custo = false;
        self.vinculo_detalhe_option = VinculoDetalheOption::Slot(VinculoImageSlot::Original);
        self.vinculo_image_slot = VinculoImageSlot::Original;
        self.refresh_vinculo_custo_input();
        self.load_vinculo_images();
        self.dados_screen = DadosScreen::VinculoDetalhe;
    }

    pub(super) fn handle_vinculo_detalhe_enter(&mut self) {
        if self.pending_unlink_vinculo {
            self.desfazer_vinculo_confirmado();
            return;
        }

        match self.vinculo_detalhe_option {
            VinculoDetalheOption::Slot(_) => self.abrir_dialogo_e_salvar_vinculo_image(),
            VinculoDetalheOption::Custo => {
                self.editing_vinculo_custo = true;
                self.db_status =
                    String::from("Editando custo do vinculo. Vazio usa o custo base do tecido.");
            }
            VinculoDetalheOption::Desfazer => {
                self.pending_unlink_vinculo = true;
                self.db_status =
                    String::from("Confirmar desfazer vinculo? S/Enter confirma, N/Esc cancela.");
            }
        }
    }

    pub(super) fn handle_desfazer_vinculo_confirmation(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('S') => {
                self.desfazer_vinculo_confirmado();
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.pending_unlink_vinculo = false;
                self.db_status = String::from("Desfazer vinculo cancelado.");
            }
            _ => {}
        }
    }

    pub(super) fn handle_vinculo_custo_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => self.salvar_vinculo_custo_override(),
            KeyCode::Esc => {
                self.editing_vinculo_custo = false;
                self.refresh_vinculo_custo_input();
                self.db_status = String::from("Edicao de custo do vinculo cancelada.");
            }
            KeyCode::Backspace => {
                self.vinculo_custo_input.pop();
            }
            KeyCode::Char(character) if !character.is_control() => {
                self.vinculo_custo_input.push(character);
            }
            _ => {}
        }
    }

    pub(super) fn select_vinculo_image_slot_shortcut(&mut self, character: char) {
        if let Some(slot) = VinculoImageSlot::from_shortcut(character) {
            self.pending_unlink_vinculo = false;
            self.editing_vinculo_custo = false;
            self.vinculo_detalhe_option = VinculoDetalheOption::Slot(slot);
            self.vinculo_image_slot = slot;
            self.refresh_vinculo_thumbnail();
        }
    }

    pub(super) fn next_vinculo_image_slot(&mut self) {
        self.pending_unlink_vinculo = false;
        self.editing_vinculo_custo = false;
        self.vinculo_detalhe_option = self.vinculo_detalhe_option.next();
        if let Some(slot) = self.vinculo_detalhe_option.selected_slot() {
            self.vinculo_image_slot = slot;
            self.refresh_vinculo_thumbnail();
        }
    }

    pub(super) fn previous_vinculo_image_slot(&mut self) {
        self.pending_unlink_vinculo = false;
        self.editing_vinculo_custo = false;
        self.vinculo_detalhe_option = self.vinculo_detalhe_option.previous();
        if let Some(slot) = self.vinculo_detalhe_option.selected_slot() {
            self.vinculo_image_slot = slot;
            self.refresh_vinculo_thumbnail();
        }
    }

    pub(super) fn navigate_vinculo_detalhe(&mut self, key: KeyCode) {
        if self.vinculos.is_empty() {
            return;
        }

        self.vinculo_lista_option = match key {
            KeyCode::BackTab => {
                (self.vinculo_lista_option + self.vinculos.len() - 1) % self.vinculos.len()
            }
            _ => (self.vinculo_lista_option + 1) % self.vinculos.len(),
        };
        self.pending_unlink_vinculo = false;
        self.editing_vinculo_custo = false;
        self.vinculo_detalhe_option = VinculoDetalheOption::Slot(VinculoImageSlot::Original);
        self.vinculo_image_slot = VinculoImageSlot::Original;
        self.refresh_vinculo_custo_input();
        self.load_vinculo_images();
        if let Some(slot) = self.first_empty_vinculo_image_slot() {
            self.vinculo_detalhe_option = VinculoDetalheOption::Slot(slot);
            self.vinculo_image_slot = slot;
            self.refresh_vinculo_thumbnail();
        }
    }

    pub(super) fn toggle_vinculo_cor(&mut self) {
        let item_id = if self.selected_vinculo_usa_estampas() {
            self.estampas
                .get(self.vinculo_criar_option)
                .map(|estampa| estampa.id)
        } else {
            self.cores.get(self.vinculo_criar_option).map(|cor| cor.id)
        };
        let Some(item_id) = item_id else { return };
        if let Some(position) = self
            .selected_vinculo_cores
            .iter()
            .position(|selected_id| *selected_id == item_id)
        {
            self.selected_vinculo_cores.remove(position);
        } else {
            self.selected_vinculo_cores.push(item_id);
        }
    }

    pub(super) fn handle_vinculo_criar_enter(&mut self) {
        let item_len = self.vinculo_item_len();
        if self.vinculo_criar_option < item_len {
            return;
        }

        if self.vinculo_criar_option == item_len {
            self.salvar_vinculos();
            self.dados_screen = DadosScreen::VinculosMenu;
        } else {
            self.voltar_dados();
        }
    }

    pub(super) fn salvar_vinculos(&mut self) {
        let Some(tecido) = self.selected_vinculo_tecido() else {
            return;
        };
        let usa_estampas = tecido.tipo == "Estampado";
        let vinculos: Vec<(i64, String)> = if usa_estampas {
            self.selected_vinculo_cores
                .iter()
                .filter_map(|estampa_id| {
                    let estampa = self
                        .estampas
                        .iter()
                        .find(|estampa| estampa.id == *estampa_id)?;
                    Some((*estampa_id, build_estampa_vinculo_sku(tecido, estampa)))
                })
                .collect()
        } else {
            self.selected_vinculo_cores
                .iter()
                .filter_map(|cor_id| {
                    let cor = self.cores.iter().find(|cor| cor.id == *cor_id)?;
                    Some((*cor_id, build_vinculo_sku(tecido, cor)))
                })
                .collect()
        };

        if let Some(pool) = &self.db_pool {
            let result = if usa_estampas {
                self.db_runtime
                    .block_on(db::replace_estampa_vinculos(pool, tecido.id, &vinculos))
            } else {
                self.db_runtime
                    .block_on(db::replace_vinculos(pool, tecido.id, &vinculos))
            };
            if let Err(error) = result {
                self.db_status = format!("Erro ao salvar vinculos: {error}");
                return;
            }
        }

        self.load_vinculos(tecido.id);
        self.db_status = String::from("Vinculos atualizados");
    }

    pub(super) fn load_vinculos(&mut self, tecido_id: i64) {
        if let Some(pool) = &self.db_pool {
            let usa_estampas = self
                .selected_vinculo_tecido()
                .map(|tecido| tecido.tipo == "Estampado")
                .unwrap_or(false);
            let result = if usa_estampas {
                self.db_runtime
                    .block_on(db::list_estampa_vinculos_by_tecido(pool, tecido_id))
            } else {
                self.db_runtime
                    .block_on(db::list_vinculos_by_tecido(pool, tecido_id))
            };
            match result {
                Ok(vinculos) => {
                    self.vinculos = vinculos;
                    self.refresh_vinculo_custo_input();
                }
                Err(error) => self.db_status = format!("Erro ao carregar vinculos: {error}"),
            }
        }
    }

    fn load_vinculo_images(&mut self) {
        let Some((tecido_id, item_id, usa_estampas)) = self.selected_vinculo_keys() else {
            return;
        };
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::get_vinculo_images(
                pool,
                tecido_id,
                item_id,
                usa_estampas,
            )) {
                Ok(images) => {
                    self.vinculo_images = images;
                    self.refresh_vinculo_thumbnail();
                }
                Err(error) => {
                    self.db_status = format!("Erro ao carregar imagens do vinculo: {error}");
                }
            }
        }
    }

    fn abrir_dialogo_e_salvar_vinculo_image(&mut self) {
        if self.vinculo_image_upload_rx.is_some() {
            self.db_status = String::from("Aguarde o upload da imagem atual terminar.");
            return;
        }

        let Some(path) = rfd::FileDialog::new()
            .set_title(self.vinculo_image_slot.title())
            .add_filter("Imagens", &["png", "jpg", "jpeg", "webp", "gif", "bmp"])
            .pick_file()
        else {
            self.db_status = String::from("Selecao de imagem cancelada.");
            return;
        };
        let Some((tecido_id, item_id, usa_estampas)) = self.selected_vinculo_keys() else {
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para salvar imagem.");
            return;
        };
        let slot = self.vinculo_image_slot;
        let pool = pool.clone();
        let (tx, rx) = mpsc::channel();

        self.vinculo_image_upload_started = Some(Instant::now());
        self.vinculo_image_upload_rx = Some(rx);
        self.db_status = format!("Salvando {}...", slot.title());

        thread::spawn(move || {
            let result =
                save_vinculo_image_upload(pool, path, tecido_id, item_id, usa_estampas, slot);
            let _ = tx.send(VinculoImageUploadResult {
                tecido_id,
                item_id,
                usa_estampas,
                slot,
                result,
            });
        });
    }

    pub(super) fn drain_vinculo_image_upload(&mut self) {
        let upload = self
            .vinculo_image_upload_rx
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        let Some(upload) = upload else {
            return;
        };

        self.vinculo_image_upload_started = None;
        self.vinculo_image_upload_rx = None;

        match upload.result {
            Ok(()) => {
                self.vinculo_thumbnail_cache.remove(&(
                    upload.tecido_id,
                    upload.item_id,
                    upload.usa_estampas,
                    upload.slot,
                ));
                self.load_vinculos(upload.tecido_id);
                self.load_vinculo_images();
                self.advance_after_vinculo_image_upload();
                self.db_status = format!("{} salva no vinculo.", upload.slot.title());
            }
            Err(error) => self.db_status = error,
        }
    }

    fn advance_after_vinculo_image_upload(&mut self) {
        if let Some(slot) = self.first_empty_vinculo_image_slot() {
            self.vinculo_detalhe_option = VinculoDetalheOption::Slot(slot);
            self.vinculo_image_slot = slot;
            self.refresh_vinculo_thumbnail();
            return;
        }

        if self.advance_to_next_incomplete_vinculo() {
            return;
        }

        self.vinculo_image_slot = VinculoImageSlot::Original;
        self.vinculo_detalhe_option = VinculoDetalheOption::Slot(VinculoImageSlot::Original);
        self.refresh_vinculo_thumbnail();
    }

    fn advance_to_next_incomplete_vinculo(&mut self) -> bool {
        if self.vinculos.is_empty() {
            return false;
        }

        let start = self.vinculo_lista_option;
        for offset in 1..=self.vinculos.len() {
            let index = (start + offset) % self.vinculos.len();
            if Self::vinculo_record_image_count(&self.vinculos[index]) < VinculoImageSlot::ALL.len()
            {
                self.vinculo_lista_option = index;
                self.vinculo_detalhe_option =
                    VinculoDetalheOption::Slot(VinculoImageSlot::Original);
                self.vinculo_image_slot = VinculoImageSlot::Original;
                self.load_vinculo_images();
                if let Some(slot) = self.first_empty_vinculo_image_slot() {
                    self.vinculo_detalhe_option = VinculoDetalheOption::Slot(slot);
                    self.vinculo_image_slot = slot;
                    self.refresh_vinculo_thumbnail();
                }
                return true;
            }
        }

        false
    }

    fn desfazer_vinculo_confirmado(&mut self) {
        let Some((tecido_id, item_id, usa_estampas)) = self.selected_vinculo_keys() else {
            self.pending_unlink_vinculo = false;
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.pending_unlink_vinculo = false;
            self.db_status = String::from("Banco local indisponivel para desfazer vinculo.");
            return;
        };

        match self.db_runtime.block_on(db::deactivate_vinculo(
            pool,
            tecido_id,
            item_id,
            usa_estampas,
        )) {
            Ok(()) => {
                self.pending_unlink_vinculo = false;
                self.load_vinculos(tecido_id);
                if self.vinculos.is_empty() {
                    self.vinculo_images = VinculoImages::default();
                    self.vinculo_thumbnail = None;
                    self.vinculo_lista_option = 0;
                    self.vinculo_custo_input.clear();
                    self.dados_screen = DadosScreen::VinculosLista;
                } else {
                    self.vinculo_lista_option = self
                        .vinculo_lista_option
                        .min(self.vinculos.len().saturating_sub(1));
                    self.vinculo_detalhe_option =
                        VinculoDetalheOption::Slot(VinculoImageSlot::Original);
                    self.vinculo_image_slot = VinculoImageSlot::Original;
                    self.refresh_vinculo_custo_input();
                    self.load_vinculo_images();
                }
                self.db_status = String::from("Vinculo desfeito para novos lancamentos.");
            }
            Err(error) => {
                self.pending_unlink_vinculo = false;
                self.db_status = format!("Erro ao desfazer vinculo: {error}");
            }
        }
    }

    fn selected_vinculo_keys(&self) -> Option<(i64, i64, bool)> {
        let tecido = self.selected_vinculo_tecido()?;
        let vinculo = self.vinculos.get(self.vinculo_lista_option)?;
        Some((tecido.id, vinculo.cor_id, tecido.tipo == "Estampado"))
    }

    fn refresh_vinculo_custo_input(&mut self) {
        self.vinculo_custo_input = self
            .vinculos
            .get(self.vinculo_lista_option)
            .and_then(|vinculo| vinculo.custo_override)
            .map(|value| format!("{value:.2}"))
            .unwrap_or_default();
    }

    fn salvar_vinculo_custo_override(&mut self) {
        let Some((tecido_id, item_id, usa_estampas)) = self.selected_vinculo_keys() else {
            self.editing_vinculo_custo = false;
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.editing_vinculo_custo = false;
            self.db_status = String::from("Banco local indisponivel para salvar custo do vinculo.");
            return;
        };

        let custo_override = if self.vinculo_custo_input.trim().is_empty() {
            None
        } else {
            match parse_number(&self.vinculo_custo_input).filter(|value| *value >= 0.0) {
                Some(value) => Some(value),
                None => {
                    self.db_status = String::from("Custo do vinculo invalido.");
                    return;
                }
            }
        };

        match self.db_runtime.block_on(db::update_vinculo_custo_override(
            pool,
            tecido_id,
            item_id,
            usa_estampas,
            custo_override,
        )) {
            Ok(()) => {
                self.editing_vinculo_custo = false;
                self.load_vinculos(tecido_id);
                self.refresh_vinculo_custo_input();
                self.db_status = if custo_override.is_some() {
                    String::from("Custo especifico do vinculo salvo.")
                } else {
                    String::from("Custo especifico removido; usando custo base do tecido.")
                };
            }
            Err(error) => {
                self.db_status = format!("Erro ao salvar custo do vinculo: {error}");
            }
        }
    }

    fn refresh_vinculo_thumbnail(&mut self) {
        self.vinculo_thumbnail = None;
        let Some((tecido_id, item_id, usa_estampas)) = self.selected_vinculo_keys() else {
            return;
        };
        let cache_key = (tecido_id, item_id, usa_estampas, self.vinculo_image_slot);

        if let Some(protocol) = self.vinculo_thumbnail_cache.get(&cache_key) {
            self.vinculo_thumbnail = Some(protocol.clone());
            return;
        }

        let Some(protocol) = self
            .selected_vinculo_image_bytes()
            .and_then(|bytes| {
                ImageReader::new(std::io::Cursor::new(bytes))
                    .with_guessed_format()
                    .ok()?
                    .decode()
                    .ok()
            })
            .and_then(|image| {
                self.image_picker
                    .new_protocol(
                        image,
                        Size::new(40, 16),
                        Resize::Fit(Some(FilterType::Lanczos3)),
                    )
                    .ok()
            })
        else {
            return;
        };

        self.vinculo_thumbnail_cache
            .insert(cache_key, protocol.clone());
        self.vinculo_thumbnail = Some(protocol);
    }

    fn selected_vinculo_image_bytes(&self) -> Option<&[u8]> {
        match self.vinculo_image_slot {
            VinculoImageSlot::Original => self.vinculo_images.imagem_original.as_deref(),
            VinculoImageSlot::Brand => self.vinculo_images.imagem_brand.as_deref(),
            VinculoImageSlot::Modelo => self.vinculo_images.imagem_modelo.as_deref(),
            VinculoImageSlot::Alternativa => self.vinculo_images.imagem_alternativa.as_deref(),
        }
    }

    fn first_empty_vinculo_image_slot(&self) -> Option<VinculoImageSlot> {
        VinculoImageSlot::ALL
            .iter()
            .copied()
            .find(|slot| !self.vinculo_images_has_slot(*slot))
    }

    fn vinculo_images_has_slot(&self, slot: VinculoImageSlot) -> bool {
        match slot {
            VinculoImageSlot::Original => self.vinculo_images.imagem_original.is_some(),
            VinculoImageSlot::Brand => self.vinculo_images.imagem_brand.is_some(),
            VinculoImageSlot::Modelo => self.vinculo_images.imagem_modelo.is_some(),
            VinculoImageSlot::Alternativa => self.vinculo_images.imagem_alternativa.is_some(),
        }
    }

    pub fn vinculo_current_image_count(&self) -> usize {
        VinculoImageSlot::ALL
            .iter()
            .filter(|slot| self.vinculo_images_has_slot(**slot))
            .count()
    }

    pub fn vinculo_record_image_count(vinculo: &VinculoRecord) -> usize {
        [
            vinculo.has_imagem_original,
            vinculo.has_imagem_brand,
            vinculo.has_imagem_modelo,
            vinculo.has_imagem_alternativa,
        ]
        .into_iter()
        .filter(|has_image| *has_image)
        .count()
    }
}

fn read_supported_image_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let mut last_error = String::new();

    for attempt in 0..6 {
        let bytes = fs::read(path).map_err(|error| format!("Erro ao ler imagem: {error}"))?;
        let expected_len = fs::metadata(path).ok().map(|metadata| metadata.len());
        if let Some(expected_len) = expected_len {
            if expected_len > bytes.len() as u64 {
                last_error = format!(
                    "Arquivo selecionado ainda esta incompleto ({} de {} bytes, {}). Aguarde o Drive sincronizar e tente novamente.",
                    bytes.len(),
                    expected_len,
                    path.display()
                );
                if attempt < 5 {
                    thread::sleep(Duration::from_millis(350));
                    continue;
                }
                break;
            }
        }

        match decode_image_bytes(&bytes) {
            Ok(_) => return Ok(bytes),
            Err(error) => {
                last_error = format!(
                    "Arquivo selecionado nao parece uma imagem suportada ({} bytes, {}): {error}",
                    bytes.len(),
                    path.display()
                );
                if is_unexpected_eof(&error) && attempt < 5 {
                    thread::sleep(Duration::from_millis(250));
                    continue;
                }
                break;
            }
        }
    }

    Err(last_error)
}

fn decode_image_bytes(bytes: &[u8]) -> Result<DynamicImage, ImageError> {
    ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()?
        .decode()
}

fn is_unexpected_eof(error: &ImageError) -> bool {
    matches!(
        error,
        ImageError::IoError(io_error) if io_error.kind() == std::io::ErrorKind::UnexpectedEof
    ) || error.to_string().contains("unexpected end of file")
}

fn save_vinculo_image_upload(
    pool: PgPool,
    path: std::path::PathBuf,
    tecido_id: i64,
    item_id: i64,
    usa_estampas: bool,
    slot: VinculoImageSlot,
) -> Result<(), String> {
    let bytes = read_supported_image_bytes(&path)?;
    if bytes.is_empty() {
        return Err(format!(
            "Arquivo selecionado esta vazio: {}",
            path.display()
        ));
    }

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| format!("Erro ao preparar upload da imagem: {error}"))?;
    runtime
        .block_on(db::update_vinculo_image(
            &pool,
            tecido_id,
            item_id,
            usa_estampas,
            slot.key(),
            &bytes,
        ))
        .map_err(|error| format!("Erro ao salvar imagem no vinculo: {error}"))
}
