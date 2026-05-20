use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::{
    db::TecidoRecord,
    shopee::{ShopeeListingUpdatePlan, ShopeeStockParentGroup},
    ui::{list_state_with_lookahead, selected_style},
};

#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    tecidos: &[TecidoRecord],
    listing_active: bool,
    listing_selected: usize,
    listing_price: &str,
    listing_confirm: bool,
    update_active: bool,
    update_selected: usize,
    update_confirm: bool,
    update_plans: &[ShopeeListingUpdatePlan],
    stock_groups: &[ShopeeStockParentGroup],
    stock_cursor: usize,
    stock_confirm: bool,
    status: &str,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(7)])
        .split(area);
    if selected == 0 && listing_active {
        render_listing_form(
            frame,
            chunks[0],
            tecidos,
            listing_selected,
            listing_price,
            listing_confirm,
        );
    } else if selected == 2 && update_active {
        render_update_form(
            frame,
            chunks[0],
            tecidos,
            update_selected,
            update_confirm,
            update_plans,
        );
    } else if selected == 1 && !stock_groups.is_empty() {
        render_stock_groups(frame, chunks[0], stock_groups, stock_cursor);
    } else {
        let items = [
            "Criar anuncio",
            "Estoque Online",
            "Atualizar anuncios",
            "Guia Shopee BR",
        ]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
        let mut state = list_state_with_lookahead(Some(selected), 4, chunks[0]);
        let list = List::new(items)
            .block(Block::default().title("Shopee").borders(Borders::ALL))
            .highlight_symbol("> ")
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());

        frame.render_stateful_widget(list, chunks[0], &mut state);
    }

    let status_text = if stock_confirm {
        format!("{status}\nEsta acao altera estoque na Shopee apenas para o SKU selecionado.")
    } else if update_confirm {
        format!("{status}\nEsta acao adiciona cores/modelos em anuncios existentes na Shopee.")
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

fn render_update_form(
    frame: &mut Frame,
    area: Rect,
    tecidos: &[TecidoRecord],
    selected: usize,
    confirm: bool,
    plans: &[ShopeeListingUpdatePlan],
) {
    let mut selected_row = 4usize;
    let mut rows = vec![
        Line::from("Atualiza anuncios existentes por SKU Pai (item_sku)."),
        Line::from(
            "Adiciona apenas cores locais faltantes, preservando tamanhos e precos remotos.",
        ),
        Line::from("Enter monta previa/confirma | Cima/Baixo seleciona tecido | Esc cancela"),
        Line::from(""),
    ];
    if tecidos.is_empty() {
        rows.push(Line::from("Nenhum tecido cadastrado."));
    } else {
        rows.extend(tecidos.iter().enumerate().map(|(index, tecido)| {
            let current = index == selected;
            if current {
                selected_row = 4 + index;
            }
            Line::from(vec![
                Span::styled(if current { "> " } else { "  " }, selected_style(current)),
                Span::styled(tecido.sku.clone(), Style::default().fg(Color::Yellow)),
                Span::raw(format!(" | {}", tecido.nome)),
            ])
        }));
    }
    if !plans.is_empty() {
        rows.push(Line::from(""));
        rows.push(Line::from("Previa:"));
        for plan in plans.iter().take(8) {
            let line = if let Some(reason) = &plan.blocked_reason {
                format!(
                    "! item {} | {} | bloqueado: {}",
                    plan.item_id, plan.item_name, reason
                )
            } else if plan.model_count == 0 {
                format!(
                    "= item {} | SKU {} | {} | {} cores x {} tamanhos | sem novas cores",
                    plan.item_id,
                    plan.parent_sku,
                    plan.item_name,
                    plan.existing_color_count,
                    plan.size_count
                )
            } else {
                format!(
                    "+ item {} | SKU {} | {} | {}+{} cores x {} tamanhos | {} modelos",
                    plan.item_id,
                    plan.parent_sku,
                    plan.item_name,
                    plan.existing_color_count,
                    plan.missing_colors.len(),
                    plan.size_count,
                    plan.model_count
                )
            };
            rows.push(Line::from(line));
        }
        if plans.len() > 8 {
            rows.push(Line::from(format!(
                "... {} anuncios a mais",
                plans.len() - 8
            )));
        }
    }
    if confirm {
        rows.push(Line::from(""));
        rows.push(Line::from(
            "Confirmar atualizacao real dos anuncios Shopee? Enter/S confirma; Esc/N cancela.",
        ));
    }
    let widget = Paragraph::new(Text::from(rows))
        .block(
            Block::default()
                .title("Shopee > Atualizar anuncios")
                .borders(Borders::ALL),
        )
        .scroll((update_scroll_offset(area, selected_row, 0) as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn update_scroll_offset(area: Rect, selected_row: usize, extra_bottom_rows: usize) -> usize {
    let visible_rows = area.height.saturating_sub(2).max(1) as usize;
    let lookahead = visible_rows.min(4);
    selected_row
        .saturating_add(lookahead)
        .saturating_add(1 + extra_bottom_rows)
        .saturating_sub(visible_rows)
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
        if !rows.is_empty() {
            rows.push(Line::from(Span::styled(
                "─".repeat(area.width.saturating_sub(4).max(20) as usize),
                Style::default().fg(Color::DarkGray),
            )));
            row_index += 1;
        }
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
            Span::styled(
                format!("{:<12}", parent.sku),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(format!(
                " {:<28} {:>3} variacoes  remoto {:>5}",
                truncate_text(&parent.name, 28),
                parent.groups.len(),
                parent.total_current_stock
            )),
        ]));
        row_index += 1;
        if parent.expanded {
            rows.push(Line::from(vec![
                Span::raw("      "),
                Span::styled(
                    format!("{:<18}", "Variacao"),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(format!("{:>8}", "Remoto"), Style::default().fg(Color::Gray)),
                Span::styled(format!("{:>8}", "Alvo"), Style::default().fg(Color::Gray)),
                Span::styled(format!("{:>8}", "Disp."), Style::default().fg(Color::Gray)),
                Span::styled(format!("{:>8}", "Res."), Style::default().fg(Color::Gray)),
                Span::raw("  Status"),
            ]));
            row_index += 1;
            for group in &parent.groups {
                let child_cursor = cursor;
                cursor += 1;
                let current = child_cursor == selected_cursor;
                if current {
                    selected_row = row_index;
                }
                let occurrence = group.occurrences.first();
                let available = occurrence.map(|item| item.available_stock).unwrap_or(0);
                let reserved = occurrence.map(|item| item.reserved_stock).unwrap_or(0);
                let status = stock_status_label(group);
                let status_style = stock_status_style(group);
                let target_style = if group.target_stock == 0 {
                    Style::default().fg(Color::Red).bold()
                } else {
                    Style::default().fg(Color::Green).bold()
                };
                rows.push(Line::from(vec![
                    Span::styled(if current { "> " } else { "  " }, selected_style(current)),
                    Span::raw("    "),
                    Span::styled(
                        format!("{:<18}", truncate_text(&group.sku, 18)),
                        Style::default().fg(Color::Green),
                    ),
                    Span::raw(format!("{:>8}", group.total_current_stock)),
                    Span::styled(format!("{:>8}", group.target_stock), target_style),
                    Span::raw(format!("{available:>8}{reserved:>8}  ")),
                    Span::styled(status, status_style),
                ]));
                row_index += 1;
                if current {
                    let mut detail_rows = 0usize;
                    if let Some(occurrence) = occurrence {
                        rows.push(Line::from(format!(
                            "       item {}  model {}  seller {}  ocorrencias {}",
                            occurrence.item_id,
                            occurrence.model_id,
                            occurrence.seller_stock,
                            group.occurrences.len()
                        )));
                        detail_rows += 1;
                    }
                    if let Some(warning) = &group.warning {
                        rows.push(Line::from(vec![
                            Span::raw("       "),
                            Span::styled(warning.clone(), Style::default().fg(Color::Red)),
                        ]));
                        detail_rows += 1;
                    }
                    row_index += detail_rows;
                }
            }
        }
    }
    let visible_rows = area.height.saturating_sub(2).max(1) as usize;
    let lookahead = visible_rows.min(4);
    let scroll_offset = selected_row
        .saturating_add(lookahead)
        .saturating_add(1)
        .saturating_sub(visible_rows)
        .min(rows.len().saturating_sub(visible_rows));
    let title = format!(
        "Shopee > Estoque Online | Enter expande/sync | Space 0/100 | C reconciliar | R recarregar | {}/{}",
        selected_cursor.saturating_add(1).min(cursor.max(1)),
        cursor.max(1)
    );
    let widget = Paragraph::new(Text::from(rows))
        .block(Block::default().title(title).borders(Borders::ALL))
        .scroll((scroll_offset as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn stock_status_label(group: &crate::shopee::ShopeeStockGroup) -> &'static str {
    if group.warning.is_some() {
        "bloqueado"
    } else if group.target_stock == 0 && group.total_current_stock == 0 {
        "zerado"
    } else if group.target_stock == 0 {
        "zerar pendente"
    } else if group.total_current_stock == group.target_stock {
        "ativo"
    } else {
        "sync pendente"
    }
}

fn stock_status_style(group: &crate::shopee::ShopeeStockGroup) -> Style {
    if group.warning.is_some() {
        Style::default().fg(Color::Red).bold()
    } else if group.target_stock == 0 && group.total_current_stock == 0 {
        Style::default().fg(Color::Gray)
    } else if group.target_stock == 0 {
        Style::default().fg(Color::Red).bold()
    } else if group.total_current_stock == group.target_stock {
        Style::default().fg(Color::Green).bold()
    } else {
        Style::default().fg(Color::Yellow).bold()
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_len {
        return text.to_string();
    }
    if max_len <= 1 {
        return String::from("~");
    }
    let mut output = text.chars().take(max_len - 1).collect::<String>();
    output.push('~');
    output
}
