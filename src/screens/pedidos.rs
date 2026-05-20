use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::{
    app::App,
    db::{PedidoRecord, TecidoRecord, VinculoRecord},
    models::{FinalizarVendaOption, PedidosScreen, VendaField, VendaItem},
    ui::{
        DIALOG_BG, SIDE_PANEL_WIDTH, color_swatch, list_state_with_lookahead,
        render_dialog_background, selected_style,
    },
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = (app.pedidos_screen == PedidosScreen::Lancamento).then(|| {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(48), Constraint::Length(SIDE_PANEL_WIDTH)])
            .split(area)
    });
    let main_area = chunks.as_ref().map(|chunks| chunks[0]).unwrap_or(area);

    match app.pedidos_screen {
        PedidosScreen::Menu => render_menu(frame, main_area, app.pedido_menu_option),
        PedidosScreen::SelecionarTecido => {
            render_tecidos(frame, main_area, app.pedido_tecido_option, &app.tecidos)
        }
        PedidosScreen::SelecionarVinculo => render_vinculos(
            frame,
            main_area,
            app.pedido_vinculo_option,
            &app.pedido_vinculos,
        ),
        PedidosScreen::Lancamento => render_lancamento(frame, main_area, app),
        PedidosScreen::Historico => render_historico(
            frame,
            main_area,
            app.pedido_historico_option,
            &app.pedidos_historico,
        ),
    }

    if let Some(chunks) = chunks {
        render_resumo(
            frame,
            chunks[1],
            &app.pedido_itens,
            app.pedido_item_option,
            app.pedido_resumo_focus,
        );
    }
    if app.pending_approve_pedido {
        render_confirm_approve(frame, area);
    }
    if app.finalizar_pedido_dialog {
        render_finalizar_pedido_dialog(frame, area, app.finalizar_pedido_option);
    }
}

fn render_menu(frame: &mut Frame, area: Rect, selected: usize) {
    let items = ["[Novo Pedido]", "[Historico de Pedidos]"]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
    let mut state = list_state_with_lookahead(Some(selected), 2, area);
    let list = List::new(items)
        .block(Block::default().title("Pedidos").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_tecidos(frame: &mut Frame, area: Rect, selected: usize, tecidos: &[TecidoRecord]) {
    let items = tecidos
        .iter()
        .enumerate()
        .map(|(index, tecido)| ListItem::new(format!("{}. {}", index + 1, tecido.nome)));
    let mut state = list_state_with_lookahead(Some(selected), tecidos.len(), area);
    let list = List::new(items)
        .block(
            Block::default()
                .title("Pedidos > Novo Pedido > Tecido")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculos(frame: &mut Frame, area: Rect, selected: usize, vinculos: &[VinculoRecord]) {
    let items = if vinculos.is_empty() {
        vec![ListItem::new("Nenhum vinculo cadastrado para este tecido.")]
    } else {
        vinculos
            .iter()
            .enumerate()
            .map(|(index, vinculo)| {
                let sku = vinculo.sku.as_deref().unwrap_or("sem-sku");
                let hex = vinculo.cor_hex.as_deref().unwrap_or("#");
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{}. {} - ", index + 1, sku)),
                    color_swatch(hex),
                    Span::raw(format!(" {}", vinculo.cor_nome)),
                ]))
            })
            .collect()
    };
    let mut state = list_state_with_lookahead(Some(selected), items.len(), area);
    let list = List::new(items)
        .block(
            Block::default()
                .title("Pedidos > Novo Pedido > Vinculo")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_lancamento(frame: &mut Frame, area: Rect, app: &App) {
    let tecido_nome = app
        .tecidos
        .get(app.pedido_tecido_option)
        .map(|tecido| tecido.nome.as_str())
        .unwrap_or("Selecione");
    let vinculo = app.pedido_vinculos.get(app.pedido_vinculo_option);
    let vinculo_nome = vinculo
        .map(|vinculo| vinculo.cor_nome.as_str())
        .unwrap_or("Selecione");
    let mut lines = Vec::new();
    lines.push(format_select(
        VendaField::Tecido,
        app.pedido_field,
        "Tecido",
        tecido_nome,
    ));
    if app.pedido_dropdown == Some(VendaField::Tecido) {
        for (index, tecido) in app.tecidos.iter().enumerate() {
            let selected = index == app.pedido_tecido_option;
            lines.push(Line::from(vec![
                Span::styled(
                    if selected { "  > " } else { "    " },
                    selected_style(selected),
                ),
                Span::styled(
                    format!("[{}]", tecido.nome),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }
    }
    lines.push(format_select(
        VendaField::Vinculo,
        app.pedido_field,
        "Vinculo",
        vinculo_nome,
    ));
    if app.pedido_dropdown == Some(VendaField::Vinculo) {
        for (index, vinculo) in app.pedido_vinculos.iter().enumerate() {
            let selected = index == app.pedido_vinculo_option;
            lines.push(Line::from(vec![
                Span::styled(
                    if selected { "  > " } else { "    " },
                    selected_style(selected),
                ),
                Span::styled(
                    format!("[{}]", vinculo.cor_nome),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }
    }
    if let Some(vinculo) = vinculo {
        lines.push(Line::from(format!(
            "SKU: {}",
            vinculo.sku.as_deref().unwrap_or("sem-sku")
        )));
    }
    lines.extend([
        Line::from(""),
        format_field(
            VendaField::Preco,
            app.pedido_field,
            "Preco Unitario",
            &app.pedido_preco,
        ),
        format_field(
            VendaField::Quantidade,
            app.pedido_field,
            "Lancar",
            &app.pedido_quantidade,
        ),
        Line::from(""),
        format_action(
            VendaField::Finalizar,
            app.pedido_field,
            if app.editing_pedido_id.is_some() {
                "[Aprovar Pedido]"
            } else {
                "[Gerar Pedido]"
            },
        ),
        format_action(VendaField::Cancelar, app.pedido_field, "[Cancelar]"),
    ]);
    if app.editing_pedido_id.is_some() {
        lines.push(format_action(
            VendaField::Excluir,
            app.pedido_field,
            "[Compartilhar]",
        ));
    }
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title(if app.editing_pedido_id.is_some() {
                "Pedidos > Historico > Pedido"
            } else {
                "Pedidos > Novo Pedido > Lancamento"
            })
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
}

fn render_historico(frame: &mut Frame, area: Rect, selected: usize, pedidos: &[PedidoRecord]) {
    let items = if pedidos.is_empty() {
        vec![ListItem::new("Nenhum pedido encontrado.")]
    } else {
        pedidos
            .iter()
            .map(|pedido| {
                let pdf = if pedido.pdf_path.is_some() {
                    " PDF"
                } else {
                    ""
                };
                ListItem::new(format!(
                    "#{}  {}  {} itens  {}{}  Total R${}",
                    pedido.id,
                    pedido.created_at,
                    pedido.itens,
                    pedido.status,
                    pdf,
                    format_money(pedido.total)
                ))
            })
            .collect()
    };
    let mut state =
        list_state_with_lookahead((!pedidos.is_empty()).then_some(selected), items.len(), area);
    let list = List::new(items)
        .block(
            Block::default()
                .title("Pedidos > Historico")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_resumo(
    frame: &mut Frame,
    area: Rect,
    itens: &[VendaItem],
    selected: usize,
    focused: bool,
) {
    let mut lines = Vec::new();
    for (index, item) in itens.iter().enumerate() {
        let current = focused && index == selected;
        lines.push(Line::from(vec![
            Span::styled(if current { "> " } else { "  " }, selected_style(current)),
            Span::raw(item.descricao.clone()),
        ]));
        lines.push(Line::from(format!(
            "  QTD: {} x R${} - Total: R${}",
            format_quantity(item.quantidade),
            format_money(item.preco_unitario),
            format_money(item.total())
        )));
        lines.push(Line::from(""));
    }
    let total = itens.iter().map(VendaItem::total).sum::<f64>();
    lines.push(Line::from(format!(
        "Total do Pedido: R${}",
        format_money(total)
    )));
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title(if focused {
                "Resumo do pedido > Del excluir"
            } else {
                "Resumo do pedido"
            })
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            }),
    );
    frame.render_widget(widget, area);
}

fn render_confirm_approve(frame: &mut Frame, area: Rect) {
    let popup = crate::ui::centered_rect(48, 7, area);
    render_dialog_background(frame, popup);
    let dialog = Paragraph::new(Text::from(vec![
        Line::from("Aprovar pedido e converter em venda?"),
        Line::from(""),
        Line::from("[Enter/S] Confirmar   [Esc/N] Voltar"),
    ]))
    .block(
        Block::default()
            .title("Confirmar pedido")
            .borders(Borders::ALL)
            .style(Style::default().bg(DIALOG_BG))
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .style(Style::default().bg(DIALOG_BG))
    .alignment(Alignment::Center);
    frame.render_widget(dialog, popup);
}

fn render_finalizar_pedido_dialog(frame: &mut Frame, area: Rect, selected: FinalizarVendaOption) {
    let popup_area = crate::ui::centered_rect(54, 8, area);
    render_dialog_background(frame, popup_area);
    let options = Line::from(vec![
        Span::styled(
            "[Finalizar]",
            selected_style(selected == FinalizarVendaOption::Finalizar),
        ),
        Span::raw("  "),
        Span::styled(
            "[Finalizar e Compartilhar]",
            selected_style(selected == FinalizarVendaOption::FinalizarEImprimir),
        ),
    ]);
    let dialog = Paragraph::new(Text::from(vec![
        Line::from("Finalizar pedido?"),
        Line::from(""),
        options,
        Line::from(""),
        Line::from("Enter confirma   Esc volta   Setas alternam"),
    ]))
    .block(
        Block::default()
            .title("Finalizar pedido")
            .borders(Borders::ALL)
            .style(Style::default().bg(DIALOG_BG))
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .style(Style::default().bg(DIALOG_BG))
    .alignment(Alignment::Center);
    frame.render_widget(dialog, popup_area);
}

fn format_select(
    field: VendaField,
    selected: VendaField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let current = field == selected;
    Line::from(vec![
        Span::styled(if current { "> " } else { "  " }, selected_style(current)),
        Span::raw(format!("{label}: ")),
        Span::styled(format!("[{value}]"), Style::default().fg(Color::Yellow)),
    ])
}

fn format_field(
    field: VendaField,
    selected: VendaField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let current = field == selected;
    Line::from(vec![
        Span::styled(if current { "> " } else { "  " }, selected_style(current)),
        Span::raw(format!("{label}: ")),
        Span::styled(
            if value.is_empty() { "_" } else { value }.to_string(),
            Style::default().fg(Color::Yellow),
        ),
    ])
}

fn format_action(field: VendaField, selected: VendaField, label: &str) -> Line<'static> {
    let current = field == selected;
    Line::from(vec![
        Span::styled(if current { "> " } else { "  " }, selected_style(current)),
        Span::raw(label.to_string()),
    ])
}

fn format_money(value: f64) -> String {
    format!("{value:.2}").replace('.', ",")
}

fn format_quantity(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format_money(value)
    }
}
