use std::process::Command;

use crossterm::event::KeyCode;

use super::App;
use crate::{db, models::Section};

impl App {
    pub(super) fn handle_configuracoes_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => self.previous_printer(),
            KeyCode::Down => self.next_printer(),
            KeyCode::Backspace if self.printer_option == self.delta_e_threshold_index() => {
                self.color_delta_e_threshold_input.pop();
            }
            KeyCode::Char(character) if self.printer_option == self.delta_e_threshold_index() => {
                if character.is_ascii_digit() || character == ',' || character == '.' {
                    self.color_delta_e_threshold_input.push(character);
                }
            }
            KeyCode::Char(' ') => {
                if let Some(printer) = self.printers.get(self.printer_option) {
                    self.selected_printer = Some(printer.clone());
                }
            }
            KeyCode::Enter => self.select_printer(),
            KeyCode::Left => self.section = self.section.previous(),
            KeyCode::Right => self.section = self.section.next(),
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
        self.printers.len() + 3
    }

    pub(super) fn delta_e_threshold_index(&self) -> usize {
        self.printers.len()
    }

    pub(super) fn select_printer(&mut self) {
        let delta_index = self.delta_e_threshold_index();
        let confirmar_index = self.printers.len() + 1;
        let voltar_index = self.printers.len() + 2;

        if self.printer_option == voltar_index {
            self.section = Section::Dashboard;
            return;
        }

        if self.printer_option == delta_index {
            return;
        }

        if self.printer_option != confirmar_index {
            return;
        }

        let selected_printer = self
            .selected_printer
            .clone()
            .or_else(|| self.printers.first().cloned());

        let Some(delta_threshold) = parse_delta_e_threshold(&self.color_delta_e_threshold_input)
        else {
            self.db_status = String::from("Informe um limiar Delta E valido maior que zero");
            return;
        };

        if let Some(pool) = &self.db_pool {
            if let Some(printer) = &selected_printer {
                if let Err(error) =
                    self.db_runtime
                        .block_on(db::set_config(pool, "receipt_printer", printer))
                {
                    self.db_status = format!("Erro ao salvar impressora: {error}");
                    return;
                }
            }
            if let Err(error) = self.db_runtime.block_on(db::set_config(
                pool,
                "color_delta_e_threshold",
                &delta_threshold.to_string(),
            )) {
                self.db_status = format!("Erro ao salvar limiar Delta E: {error}");
                return;
            }
            self.selected_printer = selected_printer.clone();
            self.color_delta_e_threshold = delta_threshold;
            self.color_delta_e_threshold_input = format_delta_e_threshold(delta_threshold);
            self.db_status = format!(
                "Configuracoes salvas. Impressora: {}; Delta E: {}",
                selected_printer.as_deref().unwrap_or("nenhuma"),
                self.color_delta_e_threshold_input
            );
        } else {
            self.db_status = String::from("Banco local indisponivel para salvar configuracoes");
        }
    }
}

pub(super) fn parse_delta_e_threshold(value: &str) -> Option<f64> {
    value
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|value| *value > 0.0)
}

fn format_delta_e_threshold(value: f64) -> String {
    format!("{value:.2}").trim_end_matches('0').trim_end_matches('.').to_string()
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
