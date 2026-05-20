use chrono::{Datelike, Days, NaiveDate};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{
        Block, Borders, Paragraph,
        calendar::{CalendarEventStore, Monthly},
    },
};
use time::{Date, Month};

use crate::{
    models::{DateRangePhase, DateRangePicker, DateRangeTarget},
    ui::{DIALOG_BG, centered_rect, render_dialog_background},
};

pub fn render(frame: &mut Frame, area: Rect, picker: Option<&DateRangePicker>) {
    let Some(picker) = picker else {
        return;
    };
    let popup_area = centered_rect(52, 15, area);
    render_dialog_background(frame, popup_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(9),
            Constraint::Length(3),
        ])
        .split(popup_area);

    let header = Paragraph::new(Text::from(vec![
        Line::from(title(picker.target)).alignment(Alignment::Center),
        Line::from(phase_label(picker.phase)).alignment(Alignment::Center),
    ]))
    .style(Style::default().bg(DIALOG_BG));
    frame.render_widget(header, chunks[0]);

    let mut events = CalendarEventStore::default();
    add_range_styles(&mut events, picker);
    let monthly = Monthly::new(to_time_date(picker.cursor), events)
        .show_month_header(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .show_weekdays_header(Style::default().fg(Color::Gray))
        .show_surrounding(Style::default().fg(Color::DarkGray))
        .default_style(Style::default().fg(Color::White).bg(DIALOG_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(DIALOG_BG)),
        );
    frame.render_widget(monthly, chunks[1]);

    let footer = Paragraph::new(Text::from(vec![
        Line::from(format!(
            "Inicio: {}   Fim: {}",
            date_label(picker.inicio),
            date_label(picker.fim)
        )),
        Line::from("Setas: dia/semana  PgUp/PgDn: mes  Home: hoje"),
        Line::from("Enter: selecionar   Esc: cancelar"),
    ]))
    .style(Style::default().bg(DIALOG_BG))
    .block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .style(Style::default().bg(DIALOG_BG)),
    );
    frame.render_widget(footer, chunks[2]);
}

fn add_range_styles(events: &mut CalendarEventStore, picker: &DateRangePicker) {
    if let Some(inicio) = picker.inicio {
        let fim = picker.fim.unwrap_or(picker.cursor);
        let (inicio, fim) = if inicio <= fim {
            (inicio, fim)
        } else {
            (fim, inicio)
        };
        let mut current = inicio;
        while current <= fim {
            events.add(
                to_time_date(current),
                Style::default().fg(Color::Black).bg(Color::Blue),
            );
            let Some(next) = current.checked_add_days(Days::new(1)) else {
                break;
            };
            current = next;
        }
    }
    events.add(
        to_time_date(picker.cursor),
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
}

fn title(target: DateRangeTarget) -> &'static str {
    match target {
        DateRangeTarget::VendasHistorico => "Selecionar periodo de vendas",
        DateRangeTarget::EstoqueResumoFornecedor => "Selecionar periodo do fornecedor",
    }
}

fn phase_label(phase: DateRangePhase) -> &'static str {
    match phase {
        DateRangePhase::Inicio => "Escolha a data inicial",
        DateRangePhase::Fim => "Escolha a data final",
    }
}

fn date_label(date: Option<NaiveDate>) -> String {
    date.map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| String::from("_"))
}

fn to_time_date(date: NaiveDate) -> Date {
    Date::from_calendar_date(
        date.year(),
        Month::try_from(date.month() as u8).unwrap_or(Month::January),
        date.day() as u8,
    )
    .unwrap_or(Date::MIN)
}
