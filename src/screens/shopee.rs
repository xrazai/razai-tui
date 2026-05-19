use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

pub fn render(frame: &mut Frame, area: Rect, selected: usize) {
    let items = ["Criar anuncio", "Estoque Online"]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(Block::default().title("Shopee").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());

    frame.render_stateful_widget(list, area, &mut state);
}
