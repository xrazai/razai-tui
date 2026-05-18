use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    app::App,
    db::{TecidoRecord, VinculoRecord},
    models::{VendaField, VendaItem, VendasScreen, parse_number},
    ui::{color_swatch, selected_style},
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(48), Constraint::Length(38)])
        .split(area);

    match app.vendas_screen {
        VendasScreen::Menu => render_menu(frame, chunks[0], app.venda_menu_option),
        VendasScreen::SelecionarTecido => {
            render_tecidos(frame, chunks[0], app.venda_tecido_option, &app.tecidos)
        }
        VendasScreen::SelecionarVinculo => render_vinculos(
            frame,
            chunks[0],
            app.venda_vinculo_option,
            &app.venda_vinculos,
        ),
        VendasScreen::Lancamento => render_lancamento(
            frame,
            chunks[0],
            app.venda_field,
            &app.venda_preco,
            &app.venda_quantidade,
        ),
        VendasScreen::Historico => {
            let widget = Paragraph::new("Historico de vendas ainda nao implementado.").block(
                Block::default()
                    .title("Vendas > Historico")
                    .borders(Borders::ALL),
            );
            frame.render_widget(widget, chunks[0]);
        }
    }

    render_resumo(frame, chunks[1], &app.venda_itens);
}

fn render_menu(frame: &mut Frame, area: Rect, selected: usize) {
    let items = ["[Nova Venda]", "[Historico de Vendas]"]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(Block::default().title("Vendas").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_tecidos(frame: &mut Frame, area: Rect, selected: usize, tecidos: &[TecidoRecord]) {
    let items = tecidos
        .iter()
        .enumerate()
        .map(|(index, tecido)| ListItem::new(format!("{}. {}", index + 1, tecido.nome)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Vendas > Nova Venda > Tecido")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculos(frame: &mut Frame, area: Rect, selected: usize, vinculos: &[VinculoRecord]) {
    let items = vinculos.iter().enumerate().map(|(index, vinculo)| {
        let sku = vinculo.sku.as_deref().unwrap_or("sem-sku");
        let hex = vinculo.cor_hex.as_deref().unwrap_or("#");
        ListItem::new(Line::from(vec![
            Span::raw(format!("{}. {} - ", index + 1, sku)),
            color_swatch(hex),
            Span::raw(format!(" {}", vinculo.cor_nome)),
        ]))
    });
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Vendas > Nova Venda > Vinculo")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_lancamento(
    frame: &mut Frame,
    area: Rect,
    field: VendaField,
    preco: &str,
    quantidade: &str,
) {
    let valid =
        parse_number(preco).unwrap_or(0.0) > 0.0 && parse_number(quantidade).unwrap_or(0.0) > 0.0;
    let lines = vec![
        format_field(VendaField::Preco, field, "Preco Unitario", preco),
        format_field(VendaField::Quantidade, field, "Lancar", quantidade),
        Line::from(""),
        format_action(field, valid),
    ];
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Vendas > Nova Venda > Lancamento")
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
}

fn render_resumo(frame: &mut Frame, area: Rect, itens: &[VendaItem]) {
    let mut lines = itens
        .iter()
        .map(|item| {
            Line::from(format!(
                "{} - {} | {:.2} x {:.2} = {:.2}",
                item.vinculo_sku,
                item.descricao,
                item.quantidade,
                item.preco_unitario,
                item.total()
            ))
        })
        .collect::<Vec<_>>();
    let total = itens.iter().map(VendaItem::total).sum::<f64>();
    lines.push(Line::from(""));
    lines.push(Line::from(format!("Total: {:.2}", total)));
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Resumo do pedido")
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
}

fn format_field(
    field: VendaField,
    selected: VendaField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let value = if value.is_empty() { "_" } else { value };
    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
    ])
}

fn format_action(selected: VendaField, valid: bool) -> Line<'static> {
    let marker = if selected == VendaField::Confirmar {
        ">"
    } else {
        " "
    };
    let suffix = if valid {
        ""
    } else {
        "  preco e quantidade obrigatorios"
    };
    Line::from(vec![
        Span::styled(
            format!("{marker} "),
            selected_style(selected == VendaField::Confirmar),
        ),
        Span::raw("[Lancar]"),
        Span::raw(suffix),
    ])
}
