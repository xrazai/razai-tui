use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use crossterm::event::KeyCode;
use windows::{
    Win32::{
        Foundation::HWND,
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    },
    core::PCWSTR,
};

use super::App;
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
                    self.pedido_field = self.pedido_field.next();
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
                    self.pedido_field = self.pedido_field.previous();
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
                    self.pedido_preco.clear();
                    self.pedido_quantidade.clear();
                }
            }
            PedidosScreen::Lancamento => {
                if self.pedido_resumo_focus {
                    return;
                }
                if matches!(self.pedido_field, VendaField::Tecido | VendaField::Vinculo) {
                    self.pedido_dropdown = Some(self.pedido_field);
                    return;
                }
                match self.pedido_field {
                    VendaField::Quantidade => self.confirmar_lancamento_pedido(),
                    VendaField::Finalizar if self.editing_pedido_id.is_some() => {
                        self.pending_approve_pedido = true
                    }
                    VendaField::Finalizar => self.abrir_dialog_finalizar_pedido(),
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
            }
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
            }
            _ => {}
        }
    }

    fn confirmar_lancamento_pedido(&mut self) {
        let Some(vinculo) = self.pedido_vinculos.get(self.pedido_vinculo_option) else {
            return;
        };
        let preco = parse_number(&self.pedido_preco).unwrap_or(0.0);
        let quantidade = parse_number(&self.pedido_quantidade).unwrap_or(0.0);
        if preco <= 0.0 || quantidade <= 0.0 {
            return;
        }
        self.pedido_itens.push(VendaItem {
            descricao: format!("{} - {}", vinculo.tecido_nome, vinculo.cor_nome),
            quantidade,
            preco_unitario: preco,
        });
        self.pedido_item_option = self.pedido_itens.len().saturating_sub(1);
        self.pedido_quantidade.clear();
        self.pedido_dropdown = None;
        self.db_status = String::from("Lancamento adicionado ao pedido");
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
        match self.gerar_pdf_pedido(pedido_id) {
            Ok(path) => {
                let path_text = path.to_string_lossy().to_string();
                let _ = self
                    .db_runtime
                    .block_on(db::update_pedido_pdf_path(pool, pedido_id, &path_text));
                if compartilhar {
                    self.db_status = match self.abrir_compartilhamento_pedido(pedido_id, &path_text)
                    {
                        Ok(()) => {
                            format!(
                                "Pedido #{pedido_id} gerado. Compartilhamento aberto: {path_text}"
                            )
                        }
                        Err(error) => format!("{error} PDF salvo em: {path_text}"),
                    };
                } else {
                    self.db_status = format!("Pedido #{pedido_id} gerado. PDF: {path_text}");
                }
            }
            Err(error) => self.db_status = format!("Pedido salvo, erro ao gerar PDF: {error}"),
        }
        self.reload_pedidos_historico();
        self.pedido_itens.clear();
        self.pedido_vinculos.clear();
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
        if let Some(pool) = &self.db_pool {
            match self
                .db_runtime
                .block_on(db::list_pedido_itens(pool, pedido_id))
            {
                Ok(itens) => {
                    self.pedido_itens = itens;
                    self.editing_pedido_id = Some(pedido_id);
                    self.pedidos_screen = PedidosScreen::Lancamento;
                    self.pedido_field = VendaField::Finalizar;
                    self.db_status = format!("Pedido #{pedido_id} aberto");
                }
                Err(error) => self.db_status = format!("Erro ao abrir pedido: {error}"),
            }
        }
    }

    fn compartilhar_pedido_atual(&mut self) {
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
            match self.gerar_pdf_pedido(pedido_id) {
                Ok(path) => {
                    let path_text = path.to_string_lossy().to_string();
                    let _ = self
                        .db_runtime
                        .block_on(db::update_pedido_pdf_path(pool, pedido_id, &path_text));
                    self.reload_pedidos_historico();
                    path_text
                }
                Err(error) => {
                    self.db_status = format!("Erro ao gerar PDF do pedido: {error}");
                    return;
                }
            }
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
        if let Some(pool) = &self.db_pool {
            match self
                .db_runtime
                .block_on(db::approve_pedido(pool, pedido_id))
            {
                Ok(()) => {
                    self.reload_pedidos_historico();
                    self.reload_vendas_historico();
                    self.pedido_itens.clear();
                    self.editing_pedido_id = None;
                    self.pending_approve_pedido = false;
                    self.pedidos_screen = PedidosScreen::Historico;
                    self.db_status = format!("Pedido #{pedido_id} aprovado e convertido em venda");
                }
                Err(error) => self.db_status = format!("Erro ao aprovar pedido: {error}"),
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
        self.pedido_preco.clear();
        self.pedido_quantidade.clear();
        self.editing_pedido_id = None;
        self.pending_approve_pedido = false;
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
            VendaField::Preco => self.pedido_preco.push(character),
            VendaField::Quantidade => self.pedido_quantidade.push(character),
            _ => {}
        }
    }

    fn backspace_pedido_field(&mut self) {
        match self.pedido_field {
            VendaField::Preco => {
                self.pedido_preco.pop();
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

    fn normalize_pedido_field(&mut self) {
        if self.editing_pedido_id.is_none() && self.pedido_field == VendaField::Excluir {
            self.pedido_field = VendaField::Tecido;
        }
    }

    fn voltar_pedidos(&mut self) {
        if self.pedidos_screen == PedidosScreen::Lancamento && self.pedido_resumo_focus {
            self.pedido_resumo_focus = false;
            return;
        }
        self.pedidos_screen = match self.pedidos_screen {
            PedidosScreen::Menu => PedidosScreen::Menu,
            PedidosScreen::SelecionarTecido | PedidosScreen::Historico => PedidosScreen::Menu,
            PedidosScreen::SelecionarVinculo => PedidosScreen::SelecionarTecido,
            PedidosScreen::Lancamento => PedidosScreen::Menu,
        };
        self.pending_approve_pedido = false;
        self.finalizar_pedido_dialog = false;
        self.pedido_dropdown = None;
        if self.pedidos_screen != PedidosScreen::Lancamento {
            self.editing_pedido_id = None;
        }
    }

    fn gerar_pdf_pedido(&self, pedido_id: i64) -> Result<PathBuf, String> {
        let dir = pedidos_pdf_dir()?;
        let path = dir.join(format!("razai_pedido_{pedido_id}.pdf"));
        pdf::write_pedido_pdf(&path, pedido_id, &self.pedido_itens)?;
        Ok(path)
    }

    fn abrir_compartilhamento_pedido(&self, pedido_id: i64, path: &str) -> Result<(), String> {
        log_share_debug(&format!(
            "Iniciando compartilhamento do pedido #{pedido_id}: {path}"
        ));
        if let Err(error) = abrir_compartilhamento_windows(path) {
            log_share_error(&format!("Pedido #{pedido_id}: {error}"));
            if let Err(fallback_error) = abrir_pdf_no_explorer(path) {
                log_share_error(&format!(
                    "Pedido #{pedido_id} fallback Explorer: {fallback_error}"
                ));
                return Err(format!(
                    "Nao foi possivel abrir o compartilhamento nativo. Fallback Explorer falhou: {fallback_error}."
                ));
            }
            return Err(String::from(
                "Nao foi possivel abrir o compartilhamento nativo.",
            ));
        }
        Ok(())
    }
}

fn pedidos_pdf_dir() -> Result<PathBuf, String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pdf_pedidos");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("falha ao criar pasta de PDFs do pedido: {error}"))?;
    Ok(dir)
}

fn abrir_compartilhamento_windows(path: &str) -> Result<(), String> {
    log_share_debug("Normalizando caminho do PDF");
    let path = Path::new(path)
        .canonicalize()
        .map_err(|error| format!("Nao foi possivel localizar o PDF do pedido: {error}"))?;
    let verb = wide_null("share");
    let file = wide_null(&path.to_string_lossy());
    log_share_debug(&format!("Chamando ShellExecuteW share: {}", path.display()));
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err(format!(
            "ShellExecuteW share falhou com codigo {}",
            result.0 as isize
        ));
    }
    Ok(())
}

fn abrir_pdf_no_explorer(path: &str) -> Result<(), String> {
    let path = Path::new(path)
        .canonicalize()
        .map_err(|error| format!("Nao foi possivel localizar o PDF do pedido: {error}"))?;
    Command::new("explorer")
        .arg(format!("/select,\"{}\"", path.display()))
        .spawn()
        .map_err(|error| format!("Nao foi possivel abrir o Explorer com o PDF: {error}"))?;
    Ok(())
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn log_share_error(message: &str) {
    let path = std::env::temp_dir().join("razai_share_error.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

fn log_share_debug(message: &str) {
    let path = std::env::temp_dir().join("razai_share_debug.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}
