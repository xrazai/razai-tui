use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::{
    app::App,
    ui::{list_state_with_action_separators, list_state_with_lookahead, selected_style},
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(7)])
        .split(area);

    if app.checklist_active {
        render_checklist(frame, chunks[0], app);
    } else {
        let items = ["Imprimir Checklist"]
            .iter()
            .enumerate()
            .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
        let mut state = list_state_with_lookahead(Some(app.documentos_option), 1, chunks[0]);
        let list = List::new(items)
            .block(Block::default().title("Documentos").borders(Borders::ALL))
            .highlight_symbol("> ")
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
        frame.render_stateful_widget(list, chunks[0], &mut state);
    }

    let status = if app.checklist_active {
        "Space marca/desmarca tecido | Ctrl+Enter gera PDF | Enter confirmar | Esc voltar"
    } else {
        "Enter abre a emissão do checklist"
    };
    let widget = Paragraph::new(format!("{status}\n{}", app.db_status))
        .block(
            Block::default()
                .title("Status Documentos")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, chunks[1]);
}

fn render_checklist(frame: &mut Frame, area: Rect, app: &App) {
    let mut items = app
        .tecidos
        .iter()
        .enumerate()
        .map(|(index, tecido)| {
            let selected = app.checklist_selected_tecidos.contains(&tecido.id);
            let current = app.checklist_cursor == index;
            ListItem::new(Line::from(vec![
                Span::styled(
                    if selected { "[+]" } else { "[ ]" },
                    selected_style(current),
                ),
                Span::raw(" "),
                Span::styled(tecido.sku.clone(), Style::default().fg(Color::Yellow)),
                Span::raw(format!(" - {}", tecido.nome)),
            ]))
        })
        .collect::<Vec<_>>();
    if app.tecidos.is_empty() {
        items.push(ListItem::new("Nenhum tecido cadastrado."));
    }

    let action_start = items.len();
    items.push(ListItem::new(""));
    items.push(ListItem::new("[Gerar PDF]"));
    items.push(ListItem::new("[Voltar]"));
    let mut state = list_state_with_action_separators(
        Some(app.checklist_cursor),
        items.len() - 1,
        area,
        &[action_start],
    );
    let list = List::new(items)
        .block(
            Block::default()
                .title("Documentos > Imprimir Checklist")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}
