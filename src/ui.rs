use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Clear, ListState},
};

use crate::models::parse_hex_color;

pub const SIDE_PANEL_WIDTH: u16 = 48;
pub const DIALOG_BG: Color = Color::Rgb(10, 12, 16);

pub fn selected_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

pub fn color_swatch(hex: &str) -> Span<'static> {
    let style = parse_hex_color(hex)
        .map(|(red, green, blue)| Style::default().fg(Color::Rgb(red, green, blue)))
        .unwrap_or_else(|| Style::default().fg(Color::DarkGray));

    Span::styled("██", style)
}

pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;

    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

pub fn render_dialog_background(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().style(Style::default().bg(DIALOG_BG)), area);
}

pub fn list_state_with_lookahead(
    selected: Option<usize>,
    item_count: usize,
    area: Rect,
) -> ListState {
    let Some(selected) = selected else {
        return ListState::default();
    };
    let visible_rows = area.height.saturating_sub(2).max(1) as usize;
    let lookahead = visible_rows.min(4);
    let max_offset = item_count.saturating_sub(visible_rows);
    let offset = selected
        .saturating_add(lookahead)
        .saturating_add(1)
        .saturating_sub(visible_rows)
        .min(max_offset);

    ListState::default()
        .with_selected(Some(selected))
        .with_offset(offset)
}

pub fn list_state_with_action_separators(
    selected: Option<usize>,
    logical_item_count: usize,
    area: Rect,
    separators_before: &[usize],
) -> ListState {
    let visual_selected = selected.map(|selected| {
        selected
            + separators_before
                .iter()
                .filter(|index| **index <= selected)
                .count()
    });
    list_state_with_lookahead(
        visual_selected,
        logical_item_count + separators_before.len(),
        area,
    )
}
