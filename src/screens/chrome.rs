use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
};

use crate::{
    agent,
    models::{ChatState, Focus, Section},
};

pub fn render_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("RAZAI TUI", Style::default().fg(Color::White).bold()),
        Span::raw("  Sistema de loja via terminal"),
    ]))
    .block(Block::default().borders(Borders::BOTTOM))
    .alignment(Alignment::Center);

    frame.render_widget(header, area);
}

pub fn render_tabs(frame: &mut Frame, area: Rect, selected: Section) {
    let titles = Section::ALL.map(|section| section.title());
    let tabs = Tabs::new(titles)
        .select(selected.index())
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());

    frame.render_widget(tabs, area);
}

pub fn render_content(frame: &mut Frame, area: Rect, selected: Section) {
    let content = Paragraph::new("")
        .block(
            Block::default()
                .title(selected.title())
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    frame.render_widget(content, area);
}

pub fn render_chat(
    frame: &mut Frame,
    area: Rect,
    chat: &ChatState,
    focus: Focus,
    context: &agent::AgentContext,
) {
    let border_style = Style::default().fg(Color::White);
    let selected_border_style = if focus == Focus::Chat {
        Style::default().fg(Color::Cyan)
    } else {
        border_style
    };
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Min(5),
            ratatui::layout::Constraint::Length(3),
        ])
        .split(area);

    let agent_panel = Paragraph::new(format!("Razai Master\nContexto: {}", context.capability))
        .block(
            Block::default()
                .title("Agente")
                .borders(Borders::ALL)
                .border_style(selected_border_style),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(agent_panel, chunks[0]);

    let history = if chat.messages.is_empty() {
        String::from("Tab foca o chat. Enter envia.")
    } else {
        chat.messages
            .iter()
            .rev()
            .take(8)
            .rev()
            .map(|message| format!("{}: {}", message.author, message.text))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let messages = Paragraph::new(history)
        .block(
            Block::default()
                .title("Chat")
                .borders(Borders::ALL)
                .border_style(selected_border_style),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(messages, chunks[1]);

    let input_text = if chat.waiting && chat.input.is_empty() {
        String::from("Consultando agente...")
    } else {
        chat.input.clone()
    };
    let input = Paragraph::new(Span::styled(input_text, Style::default().fg(Color::Yellow)))
        .block(
            Block::default()
                .title("Mensagem")
                .borders(Borders::ALL)
                .border_style(selected_border_style),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(input, chunks[2]);
}

pub fn render_footer(frame: &mut Frame, area: Rect, db_status: &str, focus: Focus) {
    let footer = Paragraph::new(format!(
        "Foco: {} | Tab alternar foco | Esq/Dir abas | Cima/Baixo selecionar | Space marcar/desmarcar | Enter abrir/confirmar | Backspace apagar | Esc voltar/cancelar | Ctrl+C sair | {db_status}",
        focus.title(),
    ))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: false });

    frame.render_widget(footer, area);
}
