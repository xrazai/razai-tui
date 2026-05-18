use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::models::parse_hex_color;

pub const SIDE_PANEL_WIDTH: u16 = 48;

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
