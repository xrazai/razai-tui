use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    db::TecidoRecord,
    shopee::ShopeeStockGroup,
    ui::selected_style,
};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    tecidos: &[TecidoRecord],
    listing_active: bool,
    listing_selected: usize,
    listing_price: &str,
    listing_confirm: bool,
    stock_groups: &[ShopeeStockGroup],
    stock_selected: usize,
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
        render_stock_groups(frame, chunks[0], stock_groups, stock_selected);
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
        rows.push(Line::from("Confirmar criacao real do anuncio NORMAL na Shopee? Enter/S confirma; Esc/N cancela."));
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
    groups: &[ShopeeStockGroup],
    selected: usize,
) {
    let rows = groups
        .iter()
        .enumerate()
        .flat_map(|(index, group)| {
            let current = index == selected;
            let warning = group
                .warning
                .as_deref()
                .map(|warning| format!(" | {warning}"))
                .unwrap_or_default();
            let first_line = Line::from(vec![
                Span::styled(if current { "> " } else { "  " }, selected_style(current)),
                Span::styled(group.sku.clone(), Style::default().fg(Color::Yellow)),
                Span::raw(format!(
                    " | {} ocorr. | atual {} | {}{}",
                    group.occurrences.len(),
                    group.total_current_stock,
                    group.target_label(),
                    warning
                )),
            ]);
            let second_line = group
                .occurrences
                .first()
                .map(|occurrence| {
                    Line::from(format!(
                        "    Ex: item {} model {} | {} | seller {} disp {} res {}",
                        occurrence.item_id,
                        occurrence.model_id,
                        occurrence.name,
                        occurrence.seller_stock,
                        occurrence.available_stock,
                        occurrence.reserved_stock
                    ))
                })
                .unwrap_or_else(|| Line::from(""));
            [first_line, second_line]
        })
        .collect::<Vec<_>>();
    let widget = Paragraph::new(Text::from(rows))
        .block(
            Block::default()
                .title("Shopee > Estoque Online | Space 0/100 | Enter sincroniza SKU | R recarregar")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}
