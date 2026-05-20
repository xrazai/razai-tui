use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::{
    app::App,
    db::{
        EstoqueMovimentoRecord, EstoqueOrdemRecord, EstoqueSaldoRecord, FornecedorRecord,
        FornecedorResumoVendaRecord, MaisVendidoRecord,
    },
    models::{EstoqueMovimentoField, EstoqueMovimentoTipo, EstoqueScreen, EstoqueView},
    ui::{list_state_with_lookahead, selected_style, table_cell, table_cell_right},
};

pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    match app.estoque_screen {
        EstoqueScreen::Menu => render_menu(frame, area, app.estoque_menu_option),
        EstoqueScreen::Lista => render_lista(frame, area, app),
        EstoqueScreen::Detalhe => render_detalhe(frame, area, app),
        EstoqueScreen::Movimento => render_movimento(frame, area, app),
        EstoqueScreen::OrdemDetalhe => render_ordem_detalhe(frame, area, app),
        EstoqueScreen::OrdemFornecedor => render_ordem_fornecedor(frame, area, app),
    }
}

fn render_lista(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    match app.estoque_view {
        EstoqueView::Saldos => render_saldos(frame, area, app.estoque_option, &app.estoque_saldos),
        EstoqueView::Ordens => {
            render_ordens(frame, area, app.estoque_ordem_option, &app.estoque_ordens)
        }
        EstoqueView::ResumoFornecedor => render_resumo_fornecedor(frame, area, app),
        EstoqueView::MaisVendidos => render_mais_vendidos(frame, area, app),
    }
}

fn render_menu(frame: &mut Frame, area: Rect, selected: usize) {
    let options = [
        "Ver todo o estoque",
        "Ver ordens de estoque",
        "Ver resumo fornecedor",
        "Ver mais vendidos",
    ];
    let items = options
        .iter()
        .enumerate()
        .map(|(index, option)| ListItem::new(format!("{}. [{}]", index + 1, option)));
    let mut state = list_state_with_lookahead(Some(selected), options.len(), area);
    let list = List::new(items)
        .block(Block::default().title("Estoque").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_saldos(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    selected: usize,
    saldos: &[EstoqueSaldoRecord],
) {
    let items = if saldos.is_empty() {
        vec![ListItem::new("Nenhum vinculo cadastrado para estoque.")]
    } else {
        let mut items = vec![ListItem::new(Line::from(format!(
            "{} {} {} {} {} {}",
            table_cell("SKU", 12),
            table_cell("Tecido", 24),
            table_cell("Cor/Estampa", 18),
            table_cell_right("Saldo", 10),
            table_cell_right("Atacado", 12),
            table_cell_right("Varejo", 12)
        )))];
        let mut previous_tecido = "";
        for saldo in saldos {
            if saldo.tecido_nome != previous_tecido {
                previous_tecido = &saldo.tecido_nome;
                items.push(tecido_separator(&saldo.tecido_nome));
            }
            items.push(saldo_line(saldo));
        }
        items
    };
    let selected_visual =
        (!saldos.is_empty()).then(|| selected + 1 + separators_before(saldos, selected));
    let mut state = list_state_with_lookahead(selected_visual, items.len(), area);
    let list = List::new(items)
        .block(
            Block::default()
                .title("Estoque > Todo o estoque")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_ordens(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    selected: usize,
    ordens: &[EstoqueOrdemRecord],
) {
    let items = if ordens.is_empty() {
        vec![ListItem::new("Nenhuma ordem de estoque cadastrada.")]
    } else {
        let mut items = vec![ListItem::new(Line::from(format!(
            "{} {} {} {} {} {} {}",
            table_cell("Data", 17),
            table_cell("SKU", 10),
            table_cell("Tecido", 20),
            table_cell("Cor/Estampa", 16),
            table_cell_right("Qtd", 9),
            table_cell("Status", 12),
            table_cell("Fornecedor", 18)
        )))];
        items.extend(ordens.iter().map(|ordem| ordem_line(ordem)));
        items
    };
    let mut state = list_state_with_lookahead(
        (!ordens.is_empty()).then_some(selected + 1),
        items.len(),
        area,
    );
    let list = List::new(items)
        .block(
            Block::default()
                .title("Estoque > Ordens")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_resumo_fornecedor(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let fornecedor = app
        .fornecedores
        .get(app.estoque_resumo_fornecedor_option)
        .map(|fornecedor| fornecedor.nome.as_str())
        .unwrap_or("Nenhum fornecedor cadastrado");
    let mut lines = vec![
        resumo_filter_line(0, app.estoque_resumo_field, "Fornecedor", fornecedor),
        resumo_filter_line(
            1,
            app.estoque_resumo_field,
            "Inicio",
            &app.estoque_resumo_inicio,
        ),
        resumo_filter_line(2, app.estoque_resumo_field, "Fim", &app.estoque_resumo_fim),
        Line::from(""),
        Line::from(format!(
            "{} {} {}",
            table_cell("Tecido", 32),
            table_cell_right("Qtd vendida", 14),
            table_cell_right("Custo vendido", 16)
        )),
    ];
    if app.estoque_resumo_fornecedor.is_empty() {
        lines.push(Line::from("Nenhuma venda encontrada no periodo."));
    } else {
        lines.extend(app.estoque_resumo_fornecedor.iter().map(resumo_venda_line));
        let quantidade = app
            .estoque_resumo_fornecedor
            .iter()
            .map(|record| record.quantidade)
            .sum::<f64>();
        let custo = app
            .estoque_resumo_fornecedor
            .iter()
            .map(|record| record.custo_total)
            .sum::<f64>();
        lines.extend([
            Line::from(""),
            Line::from(format!(
                "{} {} {}",
                table_cell("Total", 32),
                table_cell_right(&format_quantity(quantidade), 14),
                table_cell_right(&format_money(custo), 16)
            )),
        ]);
    }
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Estoque > Resumo fornecedor")
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
}

fn resumo_filter_line(field: usize, selected: usize, label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            if field == selected { "> " } else { "  " },
            selected_style(field == selected),
        ),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
    ])
}

fn resumo_venda_line(record: &FornecedorResumoVendaRecord) -> Line<'static> {
    Line::from(format!(
        "{} {} {}",
        table_cell(&record.tecido_nome, 32),
        table_cell_right(&format_quantity(record.quantidade), 14),
        table_cell_right(&format_money(record.custo_total), 16)
    ))
}

fn render_mais_vendidos(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let max = app
        .estoque_mais_vendidos
        .iter()
        .map(|record| record.quantidade)
        .fold(0.0, f64::max);
    let mut lines = Vec::new();
    if app.estoque_mais_vendidos.is_empty() {
        lines.push(Line::from("Nenhuma venda encontrada."));
    } else {
        lines.extend(
            app.estoque_mais_vendidos
                .iter()
                .map(|record| mais_vendido_line(record, max)),
        );
    }
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Estoque > Mais vendidos")
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
}

fn mais_vendido_line(record: &MaisVendidoRecord, max: f64) -> Line<'static> {
    let label = format!(
        "{} - {} {{{}}}",
        record.tecido_nome,
        record.item_nome,
        record.sku.as_deref().unwrap_or("sem-sku")
    );
    Line::from(format!(
        "{} | [{}] {} vendidos",
        table_cell(&label, 42),
        bar(record.quantidade, max, 28),
        format_quantity(record.quantidade)
    ))
}

fn bar(value: f64, max: f64, width: usize) -> String {
    let filled = if max <= 0.0 {
        0
    } else {
        ((value / max) * width as f64)
            .round()
            .clamp(1.0, width as f64) as usize
    };
    format!("{}{}", "█".repeat(filled), " ".repeat(width - filled))
}

fn render_detalhe(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let Some(saldo) = app.estoque_saldos.get(app.estoque_option) else {
        render_saldos(frame, area, app.estoque_option, &app.estoque_saldos);
        return;
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                saldo.sku.as_deref().unwrap_or("sem-sku").to_string(),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(format!(" | {} / {}", saldo.tecido_nome, saldo.item_nome)),
        ]),
        Line::from(format!("Saldo atual: {}", format_quantity(saldo.saldo))),
        Line::from(""),
        action_line(0, app.estoque_movimento_option, "[Entrada]"),
        action_line(1, app.estoque_movimento_option, "[Transferencia]"),
        action_line(2, app.estoque_movimento_option, "[Voltar]"),
        Line::from(""),
        Line::from("Historico"),
        Line::from(format!(
            "{} {} {} {} {}",
            table_cell("Data", 19),
            table_cell("Tipo", 20),
            table_cell_right("Qtd", 10),
            table_cell("Destino", 16),
            table_cell("Obs", 24)
        )),
    ];
    lines.extend(app.estoque_movimentos.iter().map(movimento_line));
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Estoque > Detalhe")
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
}

fn render_ordem_detalhe(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let Some(ordem) = app.estoque_ordens.get(app.estoque_ordem_option) else {
        render_ordens(frame, area, app.estoque_ordem_option, &app.estoque_ordens);
        return;
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                ordem.sku.as_deref().unwrap_or("sem-sku").to_string(),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(format!(" | {} / {}", ordem.tecido_nome, ordem.item_nome)),
        ]),
        Line::from(format!(
            "Quantidade faltante: {}",
            format_quantity(ordem.quantidade)
        )),
        Line::from(format!("Status: {}", ordem.status)),
        Line::from(format!(
            "Fornecedor: {}",
            ordem
                .fornecedor_nome
                .as_deref()
                .unwrap_or("nao direcionada")
        )),
        Line::from(format!(
            "Origem: {}",
            ordem
                .venda_id
                .map(|id| format!("venda #{id}"))
                .unwrap_or_else(|| String::from("-"))
        )),
        Line::from(format!(
            "Observacao: {}",
            ordem.observacao.as_deref().unwrap_or("-")
        )),
        Line::from(""),
    ];
    if matches!(ordem.status.as_str(), "pendente" | "direcionada") {
        lines.extend([
            action_line(0, app.estoque_ordem_action_option, "[Fornecedor]"),
            action_line(1, app.estoque_ordem_action_option, "[Concluir]"),
            action_line(2, app.estoque_ordem_action_option, "[Cancelar]"),
            action_line(3, app.estoque_ordem_action_option, "[Voltar]"),
        ]);
    } else {
        lines.push(action_line(0, app.estoque_ordem_action_option, "[Voltar]"));
    }
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Estoque > Ordem")
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
}

fn render_ordem_fornecedor(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let items = if app.fornecedores.is_empty() {
        vec![ListItem::new("Nenhum fornecedor cadastrado.")]
    } else {
        app.fornecedores
            .iter()
            .map(fornecedor_line)
            .collect::<Vec<_>>()
    };
    let mut state = list_state_with_lookahead(
        (!app.fornecedores.is_empty()).then_some(app.estoque_ordem_fornecedor_option),
        items.len(),
        area,
    );
    let list = List::new(items)
        .block(
            Block::default()
                .title("Estoque > Ordem > Fornecedor")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_movimento(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let title = match app.estoque_movimento_tipo {
        EstoqueMovimentoTipo::Entrada => "Estoque > Entrada",
        EstoqueMovimentoTipo::Transferencia => "Estoque > Transferencia",
    };
    let mut lines = vec![field_line(
        EstoqueMovimentoField::Quantidade,
        app.estoque_movimento_field,
        "Quantidade",
        &app.estoque_quantidade,
    )];
    if app.estoque_movimento_tipo == EstoqueMovimentoTipo::Transferencia {
        lines.push(field_line(
            EstoqueMovimentoField::Destino,
            app.estoque_movimento_field,
            "Destino",
            &app.estoque_destino,
        ));
    }
    lines.extend([
        field_line(
            EstoqueMovimentoField::Observacao,
            app.estoque_movimento_field,
            "Observacao",
            &app.estoque_observacao,
        ),
        Line::from(""),
        action_field_line(
            EstoqueMovimentoField::Confirmar,
            app.estoque_movimento_field,
            "[Confirmar]",
        ),
        action_field_line(
            EstoqueMovimentoField::Voltar,
            app.estoque_movimento_field,
            "[Voltar]",
        ),
    ]);
    let widget = Paragraph::new(Text::from(lines))
        .block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(widget, area);
}

fn action_line(field: usize, selected: usize, label: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            if field == selected { "> " } else { "  " },
            selected_style(field == selected),
        ),
        Span::raw(label.to_string()),
    ])
}

fn field_line(
    field: EstoqueMovimentoField,
    selected: EstoqueMovimentoField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let value = if value.is_empty() { "_" } else { value };
    Line::from(vec![
        Span::styled(
            if field == selected { "> " } else { "  " },
            selected_style(field == selected),
        ),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
    ])
}

fn action_field_line(
    field: EstoqueMovimentoField,
    selected: EstoqueMovimentoField,
    label: &str,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            if field == selected { "> " } else { "  " },
            selected_style(field == selected),
        ),
        Span::raw(label.to_string()),
    ])
}

fn movimento_line(movimento: &EstoqueMovimentoRecord) -> Line<'static> {
    Line::from(format!(
        "{} {} {} {} {}",
        table_cell(&movimento.created_at, 19),
        table_cell(&movimento.tipo, 20),
        table_cell_right(&format_quantity(movimento.quantidade), 10),
        table_cell(movimento.destino.as_deref().unwrap_or("-"), 16),
        table_cell(
            movimento
                .observacao
                .as_deref()
                .unwrap_or(venda_label(movimento.venda_id)),
            24
        )
    ))
}

fn saldo_line(saldo: &EstoqueSaldoRecord) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(
            format!(
                "{} ",
                table_cell(saldo.sku.as_deref().unwrap_or("sem-sku"), 12)
            ),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(format!("{} ", table_cell(&saldo.tecido_nome, 24))),
        Span::raw(format!("{} ", table_cell(&saldo.item_nome, 18))),
        Span::styled(
            format!("{} ", table_cell_right(&format_quantity(saldo.saldo), 10)),
            if saldo.saldo < 0.0 {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Cyan)
            },
        ),
        Span::raw(format!(
            "{} ",
            table_cell_right(&format_optional_money(saldo.preco_atacado), 12)
        )),
        Span::raw(table_cell_right(
            &format_optional_money(saldo.preco_varejo),
            12,
        )),
    ]))
}

fn tecido_separator(tecido_nome: &str) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled("---- ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            tecido_nome.to_string(),
            Style::default().fg(Color::Gray).bold(),
        ),
        Span::styled(" ", Style::default().fg(Color::DarkGray)),
        Span::styled("-".repeat(72), Style::default().fg(Color::DarkGray)),
    ]))
}

fn separators_before(saldos: &[EstoqueSaldoRecord], selected: usize) -> usize {
    let mut count = 0;
    let mut previous_tecido = "";
    for saldo in saldos.iter().take(selected + 1) {
        if saldo.tecido_nome != previous_tecido {
            previous_tecido = &saldo.tecido_nome;
            count += 1;
        }
    }
    count
}

fn ordem_line(ordem: &EstoqueOrdemRecord) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::raw(format!("{} ", table_cell(&ordem.created_at, 17))),
        Span::styled(
            format!(
                "{} ",
                table_cell(ordem.sku.as_deref().unwrap_or("sem-sku"), 10)
            ),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(format!("{} ", table_cell(&ordem.tecido_nome, 20))),
        Span::raw(format!("{} ", table_cell(&ordem.item_nome, 16))),
        Span::styled(
            format!(
                "{} ",
                table_cell_right(&format_quantity(ordem.quantidade), 9)
            ),
            Style::default().fg(Color::Red),
        ),
        Span::styled(
            format!("{} ", table_cell(&ordem.status, 12)),
            status_style(&ordem.status),
        ),
        Span::raw(table_cell(
            ordem.fornecedor_nome.as_deref().unwrap_or("-"),
            18,
        )),
    ]))
}

fn fornecedor_line(fornecedor: &FornecedorRecord) -> ListItem<'static> {
    ListItem::new(Line::from(format!(
        "{} {} {}",
        table_cell(&fornecedor.nome, 24),
        table_cell(&fornecedor.empresa, 24),
        table_cell(&fornecedor.telefone, 16)
    )))
}

fn status_style(status: &str) -> Style {
    match status {
        "pendente" => Style::default().fg(Color::Red).bold(),
        "direcionada" => Style::default().fg(Color::Yellow),
        "concluida" => Style::default().fg(Color::Green),
        "cancelada" => Style::default().fg(Color::DarkGray),
        _ => Style::default(),
    }
}

fn venda_label(venda_id: Option<i64>) -> &'static str {
    if venda_id.is_some() { "venda" } else { "-" }
}

fn format_optional_money(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}").replace('.', ","))
        .unwrap_or_else(|| String::from("nao definido"))
}

fn format_money(value: f64) -> String {
    format!("{value:.2}").replace('.', ",")
}

fn format_quantity(value: f64) -> String {
    format_money(value)
}
