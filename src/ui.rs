use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, ListState, Paragraph},
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

pub fn destructive_style(selected: bool) -> Style {
    let style = Style::default().fg(Color::Red);
    if selected {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

pub fn action_line<T>(field: T, selected: T, label: &str) -> Line<'static>
where
    T: Copy + Eq,
{
    let current = field == selected;
    Line::from(vec![
        Span::styled(if current { "> " } else { "  " }, selected_style(current)),
        Span::raw(label.to_string()),
    ])
}

pub fn destructive_action_line<T>(field: T, selected: T, label: &str) -> Line<'static>
where
    T: Copy + Eq,
{
    let current = field == selected;
    Line::from(vec![
        Span::styled(
            if current { "> " } else { "  " },
            destructive_style(current),
        ),
        Span::styled(label.to_string(), destructive_style(current)),
    ])
}

pub fn color_swatch(hex: &str) -> Span<'static> {
    let style = parse_hex_color(hex)
        .map(|(red, green, blue)| Style::default().fg(Color::Rgb(red, green, blue)))
        .unwrap_or_else(|| Style::default().fg(Color::DarkGray));

    Span::styled("██", style)
}

pub fn table_cell(value: &str, width: usize) -> String {
    fit_cell(value, width, false)
}

pub fn table_cell_right(value: &str, width: usize) -> String {
    fit_cell(value, width, true)
}

fn fit_cell(value: &str, width: usize, right: bool) -> String {
    let value = value.trim();
    let char_count = value.chars().count();
    let text = if char_count > width {
        let keep = width.saturating_sub(1);
        let mut truncated = value.chars().take(keep).collect::<String>();
        truncated.push('~');
        truncated
    } else {
        value.to_string()
    };
    if right {
        format!("{text:>width$}")
    } else {
        format!("{text:<width$}")
    }
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

pub fn render_destructive_confirm_dialog(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
) {
    let popup_area = centered_rect(54, 7, area);
    render_dialog_background(frame, popup_area);
    let mut lines = message
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect::<Vec<_>>();
    lines.push(Line::from(""));
    lines.push(Line::from("[Enter/S] Confirmar   [Esc/N] Voltar"));
    let dialog = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(title.to_string())
                .borders(Borders::ALL)
                .style(Style::default().bg(DIALOG_BG))
                .border_style(Style::default().fg(Color::Red)),
        )
        .style(Style::default().bg(DIALOG_BG))
        .alignment(Alignment::Center);

    frame.render_widget(dialog, popup_area);
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
