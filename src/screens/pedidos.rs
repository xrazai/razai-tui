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
        DIALOG_BG, SIDE_PANEL_WIDTH, action_line, color_swatch, destructive_action_line,
        list_state_with_lookahead, render_destructive_confirm_dialog, render_dialog_background,
        selected_style, table_cell, table_cell_right,
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
            app.editing_pedido_item,
        );
    }
    if app.pending_approve_pedido {
        render_confirm_approve(frame, area);
    }
    if app.pending_delete_pedido {
        render_destructive_confirm_dialog(
            frame,
            area,
            "Cancelar pedido",
            "Cancelar pedido e remover do historico?",
        );
    }
    if app.finalizar_pedido_dialog {
        render_finalizar_pedido_dialog(frame, area, app.finalizar_pedido_option);
    }
    if let Some(started_at) = app.pedido_pdf_started() {
        render_pedido_pdf_progress(frame, area, started_at);
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
    let items = if tecidos.is_empty() {
        vec![ListItem::new("Nenhum tecido cadastrado.")]
    } else {
        let mut items = vec![ListItem::new(Line::from(format!(
            "{} {} {} {}",
            table_cell_right("#", 4),
            table_cell("SKU", 6),
            table_cell("Tecido", 28),
            table_cell("Tipo", 12)
        )))];
        items.extend(tecidos.iter().enumerate().map(|(index, tecido)| {
            ListItem::new(Line::from(vec![
                Span::raw(format!(
                    "{} ",
                    table_cell_right(&(index + 1).to_string(), 4)
                )),
                Span::styled(
                    format!("{} ", table_cell(&tecido.sku, 6)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(format!("{} ", table_cell(&tecido.nome, 28))),
                Span::raw(table_cell(&tecido.tipo, 12)),
            ]))
        }));
        items
    };
    let mut state = list_state_with_lookahead(
        (!tecidos.is_empty()).then_some(selected + 1),
        items.len(),
        area,
    );
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
        let mut items = vec![ListItem::new(Line::from(format!(
            "{} {} {} {} {}",
            table_cell_right("#", 4),
            table_cell("SKU", 12),
            table_cell("Cor", 18),
            table_cell_right("Atacado", 12),
            table_cell_right("Varejo", 12)
        )))];
        items.extend(vinculos.iter().enumerate().map(|(index, vinculo)| {
            let sku = vinculo.sku.as_deref().unwrap_or("sem-sku");
            let hex = vinculo.cor_hex.as_deref().unwrap_or("#");
            ListItem::new(Line::from(vec![
                Span::raw(format!(
                    "{} ",
                    table_cell_right(&(index + 1).to_string(), 4)
                )),
                Span::raw(format!("{} ", table_cell(sku, 12))),
                color_swatch(hex),
                Span::raw(format!(" {} ", table_cell(&vinculo.cor_nome, 15))),
                Span::raw(format!(
                    "{} ",
                    table_cell_right(&format_optional_money(vinculo.preco_atacado_efetivo), 12)
                )),
                Span::raw(table_cell_right(
                    &format_optional_money(vinculo.preco_varejo_efetivo),
                    12,
                )),
            ]))
        }));
        items
    };
    let mut state = list_state_with_lookahead(
        (!vinculos.is_empty()).then_some(selected + 1),
        items.len(),
        area,
    );
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
        format_select(
            VendaField::Preco,
            app.pedido_field,
            "Preco Unitario",
            &preco_option_label(app.pedido_preco_option, &app.pedido_preco),
        ),
    ]);
    if app.pedido_dropdown == Some(VendaField::Preco) {
        push_preco_options(
            &mut lines,
            vinculo,
            app.pedido_preco_option,
            &app.pedido_preco,
        );
    }
    lines.extend([
        format_field(
            VendaField::Quantidade,
            app.pedido_field,
            if app.editing_pedido_item.is_some() {
                "Atualizar"
            } else {
                "Lancar"
            },
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
    ]);
    if app.editing_pedido_id.is_some() {
        lines.push(action_line(
            VendaField::Compartilhar,
            app.pedido_field,
            "[Compartilhar]",
        ));
        lines.push(destructive_action_line(
            VendaField::Cancelar,
            app.pedido_field,
            "[Cancelar Pedido]",
        ));
    } else {
        lines.push(format_action(
            VendaField::Cancelar,
            app.pedido_field,
            "[Cancelar]",
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
        let mut items = vec![ListItem::new(Line::from(format!(
            "{} {} {} {} {} {}",
            table_cell_right("#", 6),
            table_cell("Data", 19),
            table_cell_right("Itens", 7),
            table_cell("Status", 12),
            table_cell("PDF", 5),
            table_cell_right("Total", 14)
        )))];
        items.extend(pedidos.iter().map(|pedido| {
            let pdf = if pedido.pdf_path.is_some() {
                "sim"
            } else {
                "-"
            };
            ListItem::new(Line::from(format!(
                "{} {} {} {} {} {}",
                table_cell_right(&format!("#{}", pedido.id), 6),
                table_cell(&pedido.created_at.to_string(), 19),
                table_cell_right(&pedido.itens.to_string(), 7),
                table_cell(&pedido.status, 12),
                table_cell(pdf, 5),
                table_cell_right(&format!("R${}", format_money(pedido.total)), 14)
            )))
        }));
        items
    };
    let mut state = list_state_with_lookahead(
        (!pedidos.is_empty()).then_some(selected + 1),
        items.len(),
        area,
    );
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
    editing: Option<usize>,
) {
    let mut lines = Vec::new();
    for (index, item) in itens.iter().enumerate() {
        let current = focused && index == selected;
        let editing_marker = if editing == Some(index) { " *" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(if current { "> " } else { "  " }, selected_style(current)),
            Span::raw(format!("{}{}", item.descricao, editing_marker)),
        ]));
        lines.push(Line::from(format!(
            "  QTD: {} x R${} - Total: R${}",
            format_quantity(item.quantidade),
            format_money(item.preco_unitario),
            format_money(item.total())
        )));
        lines.push(Line::from(""));
    }
    let total_quantity = itens.iter().map(|item| item.quantidade).sum::<f64>();
    let total = itens.iter().map(VendaItem::total).sum::<f64>();
    lines.push(Line::from(format!(
        "QTD Total: {}",
        format_quantity(total_quantity)
    )));
    lines.push(Line::from(format!(
        "Total do Pedido: R${}",
        format_money(total)
    )));
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title(if focused {
                "Resumo do pedido > Enter editar | Del excluir"
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

fn render_pedido_pdf_progress(frame: &mut Frame, area: Rect, started_at: std::time::Instant) {
    let popup_area = crate::ui::centered_rect(52, 7, area);
    render_dialog_background(frame, popup_area);
    let spinner = ["|", "/", "-", "\\"][(started_at.elapsed().as_millis() as usize / 250) % 4];
    let dialog = Paragraph::new(Text::from(vec![
        Line::from(format!("{spinner} Gerando PDF do pedido...")),
        Line::from(""),
        Line::from("A tela continua disponivel enquanto o PDF e preparado."),
    ]))
    .block(
        Block::default()
            .title("Pedido")
            .borders(Borders::ALL)
            .style(Style::default().bg(DIALOG_BG))
            .border_style(Style::default().fg(Color::Yellow)),
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
    action_line(field, selected, label)
}

fn format_money(value: f64) -> String {
    format!("{value:.2}").replace('.', ",")
}

fn format_optional_money(value: Option<f64>) -> String {
    value
        .map(format_money)
        .unwrap_or_else(|| String::from("nao definido"))
}

fn push_preco_options(
    lines: &mut Vec<Line<'static>>,
    vinculo: Option<&VinculoRecord>,
    selected: usize,
    manual: &str,
) {
    for option in 0..3 {
        let current = option == selected;
        lines.push(Line::from(vec![
            Span::styled(
                if current { "  > " } else { "    " },
                selected_style(current),
            ),
            Span::styled(
                format!("[{}]", preco_option_label_for(vinculo, option, manual)),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }
}

fn preco_option_label(selected: usize, manual: &str) -> String {
    match selected {
        0 => format!("Atacado R$ {}", money_or_blank(manual)),
        1 => format!("Varejo R$ {}", money_or_blank(manual)),
        _ => format!("Manual R$ {}", money_or_blank(manual)),
    }
}

fn preco_option_label_for(vinculo: Option<&VinculoRecord>, option: usize, _manual: &str) -> String {
    match option {
        0 => format!(
            "Atacado R$ {}",
            option_money(vinculo.and_then(|vinculo| vinculo.preco_atacado_efetivo))
        ),
        1 => format!(
            "Varejo R$ {}",
            option_money(vinculo.and_then(|vinculo| vinculo.preco_varejo_efetivo))
        ),
        _ => String::from("Manual R$ _"),
    }
}

fn option_money(value: Option<f64>) -> String {
    value
        .map(format_money)
        .unwrap_or_else(|| String::from("nao definido"))
}

fn money_or_blank(value: &str) -> String {
    if value.is_empty() {
        String::from("_")
    } else {
        value.to_string()
    }
}

fn format_quantity(value: f64) -> String {
    format_money(value)
}
