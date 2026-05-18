use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::{app::App, ui::selected_style};

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

    items.push(ListItem::new(Line::from("")));
    items.push(action_line(
        app.printer_option == app.printers.len(),
        "[Confirmar]",
    ));
    items.push(action_line(
        app.printer_option == app.printers.len() + 1,
        "[Voltar]",
    ));

    let mut state = ListState::default().with_selected(Some(app.printer_option));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Configuracoes > Impressora de recibos 80mm")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());

    frame.render_stateful_widget(list, area, &mut state);
}

fn action_line(selected: bool, label: &'static str) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(if selected { "> " } else { "  " }, selected_style(selected)),
        Span::raw(label),
    ]))
}
