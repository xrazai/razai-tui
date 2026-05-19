use std::{fs, path::Path};

use super::App;
use crate::{
    db::{self, TecidoRecord},
    models::*,
};
use crossterm::event::KeyCode;
use image::ImageReader;
use ratatui::layout::Size;
use ratatui_image::{Resize, picker::Picker};

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
                self.vinculo_image_upload_active = false;
                self.vinculo_image_path_input.clear();
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
        self.vinculo_image_slot = VinculoImageSlot::Original;
        self.vinculo_image_path_input.clear();
        self.vinculo_image_upload_active = false;
        self.load_vinculo_images();
        self.dados_screen = DadosScreen::VinculoDetalhe;
    }

    pub(super) fn handle_vinculo_detalhe_enter(&mut self) {
        if self.vinculo_image_upload_active {
            self.salvar_vinculo_image_upload();
            return;
        }
        self.vinculo_image_path_input.clear();
        self.vinculo_image_upload_active = true;
        self.db_status = format!(
            "Cole o caminho do arquivo para {} e pressione Enter.",
            self.vinculo_image_slot.title()
        );
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
                Ok(vinculos) => self.vinculos = vinculos,
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

    fn salvar_vinculo_image_upload(&mut self) {
        let path_text = self
            .vinculo_image_path_input
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        if path_text.is_empty() {
            self.db_status = String::from("Informe o caminho da imagem.");
            return;
        }
        let bytes = match fs::read(Path::new(&path_text)) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.db_status = format!("Erro ao ler imagem: {error}");
                return;
            }
        };
        let reader = match ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format() {
            Ok(reader) => reader,
            Err(error) => {
                self.db_status =
                    format!("Arquivo selecionado nao parece uma imagem suportada: {error}");
                return;
            }
        };
        if let Err(error) = reader.decode() {
            self.db_status =
                format!("Arquivo selecionado nao parece uma imagem suportada: {error}");
            return;
        }
        let Some((tecido_id, item_id, usa_estampas)) = self.selected_vinculo_keys() else {
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para salvar imagem.");
            return;
        };
        let slot = self.vinculo_image_slot;
        match self.db_runtime.block_on(db::update_vinculo_image(
            pool,
            tecido_id,
            item_id,
            usa_estampas,
            slot.key(),
            &bytes,
        )) {
            Ok(()) => {
                self.vinculo_image_upload_active = false;
                self.vinculo_image_path_input.clear();
                self.load_vinculos(tecido_id);
                self.load_vinculo_images();
                self.db_status = format!("{} salva no vinculo.", slot.title());
            }
            Err(error) => self.db_status = format!("Erro ao salvar imagem no vinculo: {error}"),
        }
    }

    fn selected_vinculo_keys(&self) -> Option<(i64, i64, bool)> {
        let tecido = self.selected_vinculo_tecido()?;
        let vinculo = self.vinculos.get(self.vinculo_lista_option)?;
        Some((tecido.id, vinculo.cor_id, tecido.tipo == "Estampado"))
    }

    fn refresh_vinculo_thumbnail(&mut self) {
        self.vinculo_thumbnail = self
            .vinculo_images
            .imagem_original
            .as_deref()
            .and_then(|bytes| {
                ImageReader::new(std::io::Cursor::new(bytes))
                    .with_guessed_format()
                    .ok()?
                    .decode()
                    .ok()
            })
            .and_then(|image| {
                Picker::halfblocks()
                    .new_protocol(image, Size::new(24, 10), Resize::Fit(None))
                    .ok()
            });
    }
}
