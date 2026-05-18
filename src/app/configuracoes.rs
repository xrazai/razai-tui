use std::process::Command;

use crossterm::event::KeyCode;

use super::App;
use crate::{db, models::Section};

impl App {
    pub(super) fn handle_configuracoes_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => self.previous_printer(),
            KeyCode::Down => self.next_printer(),
            KeyCode::Char(' ') => {
                if let Some(printer) = self.printers.get(self.printer_option) {
                    self.selected_printer = Some(printer.clone());
                }
            }
            KeyCode::Enter => self.select_printer(),
            KeyCode::Left | KeyCode::BackTab => self.section = self.section.previous(),
            KeyCode::Right | KeyCode::Tab => self.section = self.section.next(),
            KeyCode::Esc => self.section = Section::Dashboard,
            _ => {}
        }
    }

    pub(super) fn next_printer(&mut self) {
        self.printer_option = (self.printer_option + 1) % self.printer_menu_len();
    }

    pub(super) fn previous_printer(&mut self) {
        let len = self.printer_menu_len();
        self.printer_option = (self.printer_option + len - 1) % len;
    }

    pub(super) fn printer_menu_len(&self) -> usize {
        self.printers.len() + 2
    }

    pub(super) fn select_printer(&mut self) {
        let confirmar_index = self.printers.len();
        let voltar_index = self.printers.len() + 1;

        if self.printer_option == voltar_index {
            self.section = Section::Dashboard;
            return;
        }

        if self.printer_option != confirmar_index {
            return;
        }

        let selected_printer = self
            .selected_printer
            .clone()
            .or_else(|| self.printers.first().cloned());

        let Some(printer) = selected_printer else {
            self.db_status = String::from("Selecione uma impressora antes de confirmar");
            return;
        };

        if let Some(pool) = &self.db_pool {
            match self
                .db_runtime
                .block_on(db::set_config(pool, "receipt_printer", &printer))
            {
                Ok(()) => {
                    self.selected_printer = Some(printer.clone());
                    self.db_status = format!("Impressora de recibos: {printer}");
                }
                Err(error) => {
                    self.db_status = format!("Erro ao salvar impressora: {error}");
                }
            }
        } else {
            self.db_status = String::from("Banco local indisponivel para salvar configuracoes");
        }
    }
}

pub(super) fn list_installed_printers() -> Vec<String> {
    let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-Printer | Sort-Object Name | Select-Object -ExpandProperty Name",
        ])
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
