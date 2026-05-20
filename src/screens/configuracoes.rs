use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem},
};

use crate::{
    app::App,
    ui::{list_state_with_lookahead, selected_style},
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let mut items = app
        .printers
        .iter()
        .enumerate()
        .map(|(index, printer)| {
            let selected = app.selected_printer.as_deref() == Some(printer.as_str());
            ListItem::new(Line::from(vec![
                Span::raw(format!("{}. ", index + 1)),
                Span::styled(
                    if selected { "[x] " } else { "[ ] " },
                    selected_style(app.printer_option == index),
                ),
                Span::raw(printer.clone()),
                if selected {
                    Span::styled("  selecionada", Style::default().fg(Color::Yellow))
                } else {
                    Span::raw("")
                },
            ]))
        })
        .collect::<Vec<_>>();

    if app.printers.is_empty() {
        items.push(ListItem::new(Text::from(vec![
            Line::from("Nenhuma impressora encontrada."),
            Line::from("Instale/configure a impressora termica 80mm no Windows e reinicie o app."),
        ])));
    }

    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            if app.printer_option == app.printers.len() {
                "> "
            } else {
                "  "
            },
            selected_style(app.printer_option == app.printers.len()),
        ),
        Span::raw("Limiar Delta E: "),
        Span::styled(
            app.color_delta_e_threshold_input.clone(),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  bloqueia cores mais proximas que este valor"),
    ])));
    items.push(ListItem::new(""));
    items.push(action_line(
        app.printer_option == app.printers.len() + 1,
        "[Confirmar]",
    ));
    items.push(action_line(
        app.printer_option == app.printers.len() + 2,
        "[Voltar]",
    ));

    let empty_message_rows = usize::from(app.printers.is_empty());
    let visual_selected = app.printer_option
        + empty_message_rows
        + usize::from(app.printer_option >= app.printers.len() + 1);
    let mut state = list_state_with_lookahead(Some(visual_selected), items.len(), area);
    let list = List::new(items)
        .block(
            Block::default()
                .title("Configuracoes > Impressora e cores")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());

    frame.render_stateful_widget(list, area, &mut state);
}

fn action_line(selected: bool, label: &'static str) -> ListItem<'static> {
    let line = Line::from(vec![
        Span::styled(if selected { "> " } else { "  " }, selected_style(selected)),
        Span::raw(label),
    ]);
    ListItem::new(line)
}
