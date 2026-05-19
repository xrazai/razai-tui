use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{db::TecidoRecord, shopee::ShopeeStockParentGroup, ui::selected_style};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    tecidos: &[TecidoRecord],
    listing_active: bool,
    listing_selected: usize,
    listing_price: &str,
    listing_confirm: bool,
    stock_groups: &[ShopeeStockParentGroup],
    stock_cursor: usize,
    stock_confirm: bool,
    status: &str,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(7)])
        .split(area);
    let items = ["Criar anuncio", "Estoque Online", "Guia Shopee BR"]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(Block::default().title("Shopee").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());

    frame.render_stateful_widget(list, chunks[0], &mut state);

    if selected == 0 && listing_active {
        render_listing_form(
            frame,
            chunks[0],
            tecidos,
            listing_selected,
            listing_price,
            listing_confirm,
        );
    } else if selected == 1 && !stock_groups.is_empty() {
        render_stock_groups(frame, chunks[0], stock_groups, stock_cursor);
    }

    let status_text = if stock_confirm {
        format!("{status}\nEsta acao altera estoque na Shopee apenas para o SKU selecionado.")
    } else {
        status.to_string()
    };
    let status = Paragraph::new(status_text)
        .block(
            Block::default()
                .title("Status Shopee")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(status, chunks[1]);
}

fn render_listing_form(
    frame: &mut Frame,
    area: Rect,
    tecidos: &[TecidoRecord],
    selected: usize,
    price: &str,
    confirm: bool,
) {
    let mut rows = vec![
        Line::from("Categoria: Roupas Femininas > Tecidos > Outros"),
        Line::from("Marca: Razai Tecidos | Condicao: Novo | Status: NORMAL"),
        Line::from(format!(
            "Preco por metro: {}",
            if price.is_empty() { "<digite>" } else { price }
        )),
        Line::from("Enter seleciona/confirma | digite preco | Backspace apaga | Esc cancela"),
        Line::from(""),
    ];
    if confirm {
        rows.push(Line::from(
            "Confirmar criacao real do anuncio NORMAL na Shopee? Enter/S confirma; Esc/N cancela.",
        ));
    } else if tecidos.is_empty() {
        rows.push(Line::from("Nenhum tecido cadastrado."));
    } else {
        rows.extend(tecidos.iter().enumerate().map(|(index, tecido)| {
            let current = index == selected;
            Line::from(vec![
                Span::styled(if current { "> " } else { "  " }, selected_style(current)),
                Span::styled(tecido.sku.clone(), Style::default().fg(Color::Yellow)),
                Span::raw(format!(
                    " | {} | gramatura linear {} g/m",
                    tecido.nome,
                    tecido
                        .gramatura_linear_g_m
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| String::from("ausente"))
                )),
            ])
        }));
    }
    let widget = Paragraph::new(Text::from(rows))
        .block(
            Block::default()
                .title("Shopee > Criar anuncio")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn render_stock_groups(
    frame: &mut Frame,
    area: Rect,
    groups: &[ShopeeStockParentGroup],
    selected_cursor: usize,
) {
    let mut cursor = 0usize;
    let mut row_index = 0usize;
    let mut selected_row = 0usize;
    let mut rows = Vec::new();
    for parent in groups {
        let parent_cursor = cursor;
        cursor += 1;
        let current = parent_cursor == selected_cursor;
        if current {
            selected_row = row_index;
        }
        let marker = if parent.expanded { "[-]" } else { "[+]" };
        rows.push(Line::from(vec![
            Span::styled(if current { "> " } else { "  " }, selected_style(current)),
            Span::styled(marker, Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(parent.sku.clone(), Style::default().fg(Color::Yellow)),
            Span::raw(format!(
                " | {} variacoes | atual {} | {}",
                parent.groups.len(),
                parent.total_current_stock,
                parent.name
            )),
        ]));
        row_index += 1;
        if parent.expanded {
            for group in &parent.groups {
                let child_cursor = cursor;
                cursor += 1;
                let current = child_cursor == selected_cursor;
                if current {
                    selected_row = row_index;
                }
                let warning = group
                    .warning
                    .as_deref()
                    .map(|warning| format!(" | {warning}"))
                    .unwrap_or_default();
                rows.push(Line::from(vec![
                    Span::styled(if current { "> " } else { "  " }, selected_style(current)),
                    Span::raw("    "),
                    Span::styled(group.sku.clone(), Style::default().fg(Color::Green)),
                    Span::raw(format!(
                        " | {} ocorr. | atual {} | {}{}",
                        group.occurrences.len(),
                        group.total_current_stock,
                        group.target_label(),
                        warning
                    )),
                ]));
                row_index += 1;
                if let Some(occurrence) = group.occurrences.first() {
                    rows.push(Line::from(format!(
                        "       Ex: item {} model {} | seller {} disp {} res {}",
                        occurrence.item_id,
                        occurrence.model_id,
                        occurrence.seller_stock,
                        occurrence.available_stock,
                        occurrence.reserved_stock
                    )));
                    row_index += 1;
                }
            }
        }
    }
    let visible_rows = area.height.saturating_sub(2).max(1) as usize;
    let scroll_offset = selected_row
        .saturating_add(1)
        .saturating_sub(visible_rows)
        .min(rows.len().saturating_sub(visible_rows));
    let title = format!(
        "Shopee > Estoque Online | Enter expande/sync | Space 0/100 | R recarregar | {}/{}",
        selected_cursor.saturating_add(1).min(cursor.max(1)),
        cursor.max(1)
    );
    let widget = Paragraph::new(Text::from(rows))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL),
        )
        .scroll((scroll_offset as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}
