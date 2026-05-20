use chrono::{Days, Months, NaiveDate};
use crossterm::event::KeyCode;

use super::App;
use crate::{db, models::*};

impl App {
    pub(super) fn open_date_range_picker(&mut self, target: DateRangeTarget) {
        let (inicio_text, fim_text) = match target {
            DateRangeTarget::VendasHistorico => {
                (&self.venda_historico_inicio, &self.venda_historico_fim)
            }
            DateRangeTarget::EstoqueResumoFornecedor => {
                (&self.estoque_resumo_inicio, &self.estoque_resumo_fim)
            }
        };
        let inicio = db::parse_sales_date(inicio_text).unwrap_or_else(db::today_sales_date);
        let fim = db::parse_sales_date(fim_text).unwrap_or(inicio);
        let (inicio, fim) = normalize_date_range(inicio, fim);
        self.date_range_picker = Some(DateRangePicker {
            target,
            cursor: inicio,
            inicio: Some(inicio),
            fim: Some(fim),
            phase: DateRangePhase::Inicio,
        });
    }

    pub(super) fn handle_date_range_picker_key(&mut self, key: KeyCode) {
        let Some(picker) = self.date_range_picker.as_mut() else {
            return;
        };
        match key {
            KeyCode::Esc => self.date_range_picker = None,
            KeyCode::Left => picker.cursor = shift_days(picker.cursor, -1),
            KeyCode::Right => picker.cursor = shift_days(picker.cursor, 1),
            KeyCode::Up => picker.cursor = shift_days(picker.cursor, -7),
            KeyCode::Down => picker.cursor = shift_days(picker.cursor, 7),
            KeyCode::PageUp => picker.cursor = shift_months(picker.cursor, -1),
            KeyCode::PageDown => picker.cursor = shift_months(picker.cursor, 1),
            KeyCode::Home => picker.cursor = db::today_sales_date(),
            KeyCode::Enter => self.confirm_date_range_picker_step(),
            _ => {}
        }
    }

    fn confirm_date_range_picker_step(&mut self) {
        let Some(picker) = self.date_range_picker.as_mut() else {
            return;
        };
        match picker.phase {
            DateRangePhase::Inicio => {
                picker.inicio = Some(picker.cursor);
                picker.fim = None;
                picker.phase = DateRangePhase::Fim;
            }
            DateRangePhase::Fim => {
                picker.fim = Some(picker.cursor);
                let target = picker.target;
                let inicio = picker.inicio.unwrap_or(picker.cursor);
                let fim = picker.fim.unwrap_or(picker.cursor);
                let (inicio, fim) = normalize_date_range(inicio, fim);
                self.apply_date_range(target, inicio, fim);
                self.date_range_picker = None;
            }
        }
    }

    fn apply_date_range(&mut self, target: DateRangeTarget, inicio: NaiveDate, fim: NaiveDate) {
        let inicio = db::format_sales_date(inicio);
        let fim = db::format_sales_date(fim);
        match target {
            DateRangeTarget::VendasHistorico => {
                self.venda_historico_inicio = inicio;
                self.venda_historico_fim = fim;
                self.reload_vendas_historico();
            }
            DateRangeTarget::EstoqueResumoFornecedor => {
                self.estoque_resumo_inicio = inicio;
                self.estoque_resumo_fim = fim;
                self.reload_estoque_resumo_fornecedor();
            }
        }
    }
}

fn normalize_date_range(inicio: NaiveDate, fim: NaiveDate) -> (NaiveDate, NaiveDate) {
    if inicio <= fim {
        (inicio, fim)
    } else {
        (fim, inicio)
    }
}

fn shift_days(date: NaiveDate, amount: i64) -> NaiveDate {
    if amount >= 0 {
        date.checked_add_days(Days::new(amount as u64))
            .unwrap_or(date)
    } else {
        date.checked_sub_days(Days::new(amount.unsigned_abs()))
            .unwrap_or(date)
    }
}

fn shift_months(date: NaiveDate, amount: i32) -> NaiveDate {
    if amount >= 0 {
        date.checked_add_months(Months::new(amount as u32))
            .unwrap_or(date)
    } else {
        date.checked_sub_months(Months::new(amount.unsigned_abs()))
            .unwrap_or(date)
    }
}
