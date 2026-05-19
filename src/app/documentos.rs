use std::{fs, path::PathBuf};

use crossterm::event::KeyCode;

use super::App;
use crate::{db, models::Section};

mod pdf;

impl App {
    pub(super) fn handle_documentos_key(&mut self, key: KeyCode) {
        if self.checklist_active {
            self.handle_checklist_key(key);
            return;
        }

        match key {
            KeyCode::Esc => self.section = Section::Dashboard,
            KeyCode::Up | KeyCode::Down => self.documentos_option = 0,
            KeyCode::Enter => self.open_checklist(),
            _ => {}
        }
    }

    fn handle_checklist_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.checklist_active = false;
                self.checklist_cursor = 0;
            }
            KeyCode::Up if self.checklist_len() > 0 => {
                self.checklist_cursor =
                    (self.checklist_cursor + self.checklist_len() - 1) % self.checklist_len();
            }
            KeyCode::Down if self.checklist_len() > 0 => {
                self.checklist_cursor = (self.checklist_cursor + 1) % self.checklist_len();
            }
            KeyCode::Char(' ') => self.toggle_checklist_tecido(),
            KeyCode::Enter => {
                let gerar_index = self.tecidos.len();
                let voltar_index = self.tecidos.len() + 1;
                if self.checklist_cursor == gerar_index {
                    self.gerar_checklist_pdf();
                } else if self.checklist_cursor == voltar_index {
                    self.checklist_active = false;
                    self.checklist_cursor = 0;
                }
            }
            _ => {}
        }
    }

    pub(super) fn save_checklist_shortcut(&mut self) {
        if self.section == Section::Documentos && self.checklist_active {
            self.gerar_checklist_pdf();
        }
    }

    fn open_checklist(&mut self) {
        self.checklist_active = true;
        self.checklist_cursor = 0;
        self.checklist_selected_tecidos.clear();
        self.db_status = String::from("Selecione os tecidos para o checklist.");
    }

    fn checklist_len(&self) -> usize {
        self.tecidos.len() + 2
    }

    fn toggle_checklist_tecido(&mut self) {
        let Some(tecido) = self.tecidos.get(self.checklist_cursor) else {
            return;
        };
        if let Some(position) = self
            .checklist_selected_tecidos
            .iter()
            .position(|id| *id == tecido.id)
        {
            self.checklist_selected_tecidos.remove(position);
        } else {
            self.checklist_selected_tecidos.push(tecido.id);
        }
    }

    fn gerar_checklist_pdf(&mut self) {
        let selected = self
            .tecidos
            .iter()
            .filter(|tecido| self.checklist_selected_tecidos.contains(&tecido.id))
            .cloned()
            .collect::<Vec<_>>();
        if selected.is_empty() {
            self.db_status = String::from("Selecione ao menos um tecido para gerar o checklist.");
            return;
        }
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para gerar checklist.");
            return;
        };

        let mut sections = Vec::new();
        for tecido in selected {
            let result = if tecido.tipo == "Estampado" {
                self.db_runtime
                    .block_on(db::list_estampa_vinculos_by_tecido(pool, tecido.id))
            } else {
                self.db_runtime
                    .block_on(db::list_vinculos_by_tecido(pool, tecido.id))
            };
            match result {
                Ok(vinculos) => sections.push(pdf::ChecklistSection { tecido, vinculos }),
                Err(error) => {
                    self.db_status = format!("Erro ao carregar vinculos do checklist: {error}");
                    return;
                }
            }
        }

        let path = match checklist_pdf_path() {
            Ok(path) => path,
            Err(error) => {
                self.db_status = error;
                return;
            }
        };

        match pdf::write_checklist_pdf(&path, &sections) {
            Ok(()) => {
                self.db_status = format!("Checklist gerado em {}", path.display());
                self.checklist_active = false;
                self.checklist_cursor = 0;
            }
            Err(error) => self.db_status = format!("Erro ao gerar checklist: {error}"),
        }
    }
}

fn checklist_pdf_path() -> Result<PathBuf, String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pdf_documentos");
    fs::create_dir_all(&dir).map_err(|error| format!("falha ao criar pasta de PDFs: {error}"))?;
    Ok(dir.join(format!(
        "razai_checklist_{}.pdf",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    )))
}
