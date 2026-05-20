use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::{
    app::App,
    db::{TecidoRecord, VinculoRecord},
    models::{FinalizarVendaOption, VendaField, VendaItem, VendasScreen},
    ui::{
        DIALOG_BG, SIDE_PANEL_WIDTH, centered_rect, color_swatch, destructive_action_line,
        list_state_with_lookahead, render_destructive_confirm_dialog, render_dialog_background,
        selected_style, table_cell, table_cell_right,
    },
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = (app.vendas_screen == VendasScreen::Lancamento).then(|| {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(48), Constraint::Length(SIDE_PANEL_WIDTH)])
            .split(area)
    });
    let main_area = chunks.as_ref().map(|chunks| chunks[0]).unwrap_or(area);

    match app.vendas_screen {
        VendasScreen::Menu => render_menu(frame, main_area, app.venda_menu_option),
        VendasScreen::SelecionarTecido => {
            render_tecidos(frame, main_area, app.venda_tecido_option, &app.tecidos)
        }
        VendasScreen::SelecionarVinculo => render_vinculos(
            frame,
            main_area,
            app.venda_vinculo_option,
            &app.venda_vinculos,
        ),
        VendasScreen::Lancamento => render_lancamento(
            frame,
            main_area,
            app,
            app.venda_field,
            &app.venda_preco,
            &app.venda_quantidade,
            app.venda_vinculos.get(app.venda_vinculo_option),
        ),
        VendasScreen::Historico => {
            render_historico(
                frame,
                main_area,
                &app.vendas_historico,
                app.venda_historico_option,
                app.venda_historico_field,
                &app.venda_historico_inicio,
                &app.venda_historico_fim,
            );
        }
    }

    if let Some(chunks) = chunks {
        render_resumo(
            frame,
            chunks[1],
            &app.venda_itens,
            app.venda_item_option,
            app.venda_resumo_focus,
            app.editing_venda_item,
        );
    }
    if app.finalizar_venda_dialog {
        render_finalizar_dialog(
            frame,
            area,
            app.finalizar_venda_option,
            app.editing_venda_id.is_some(),
        );
    }
    if app.pending_delete_venda {
        render_destructive_confirm_dialog(frame, area, "Confirmar exclusao", "Excluir esta venda?");
    }
    if app.pending_delete_venda_item {
        render_destructive_confirm_dialog(
            frame,
            area,
            "Confirmar exclusao",
            "Excluir este lancamento?",
        );
    }
}

fn render_historico(
    frame: &mut Frame,
    area: Rect,
    vendas: &[crate::db::VendaHistoricoRecord],
    selected: usize,
    selected_field: usize,
    inicio: &str,
    fim: &str,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(5)])
        .split(area);

    let filters = Paragraph::new(Text::from(vec![
        format_history_field(0, selected_field, "Data inicio", inicio),
        format_history_field(1, selected_field, "Data fim", fim),
    ]))
    .block(
        Block::default()
            .title("Vendas > Historico > Periodo")
            .borders(Borders::ALL),
    );
    frame.render_widget(filters, chunks[0]);

    let items = if vendas.is_empty() {
        vec![ListItem::new("Nenhuma venda finalizada.")]
    } else {
        let mut items = vec![ListItem::new(Line::from(format!(
            "{} {} {} {}",
            table_cell_right("#", 6),
            table_cell("Data", 19),
            table_cell_right("Itens", 7),
            table_cell_right("Total", 14)
        )))];
        items.extend(vendas.iter().map(|venda| {
            ListItem::new(Line::from(format!(
                "{} {} {} {}",
                table_cell_right(&format!("#{}", venda.id), 6),
                table_cell(&venda.created_at.to_string(), 19),
                table_cell_right(&venda.itens.to_string(), 7),
                table_cell_right(&format!("R${}", format_money(venda.total)), 14)
            )))
        }));
        items
    };
    let mut state = list_state_with_lookahead(
        (!vendas.is_empty() && selected_field == 2).then_some(selected + 1),
        items.len(),
        chunks[1],
    );
    let list = List::new(items)
        .block(
            Block::default()
                .title("Vendas do periodo")
                .borders(Borders::ALL)
                .border_style(if selected_field == 2 {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                }),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, chunks[1], &mut state);
}

fn format_history_field(field: usize, selected: usize, label: &str, value: &str) -> Line<'static> {
    let is_selected = field == selected;
    Line::from(vec![
        Span::styled(
            if is_selected { "> " } else { "  " },
            selected_style(is_selected),
        ),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
    ])
}

fn render_menu(frame: &mut Frame, area: Rect, selected: usize) {
    let items = ["[Nova Venda]", "[Historico de Vendas]"]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
    let mut state = list_state_with_lookahead(Some(selected), 2, area);
    let list = List::new(items)
        .block(Block::default().title("Vendas").borders(Borders::ALL))
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
                .title("Vendas > Nova Venda > Tecido")
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
    app: &App,
    field: VendaField,
    preco: &str,
    quantidade: &str,
    vinculo: Option<&VinculoRecord>,
) {
    let mut lines = Vec::new();
    let tecido_nome = app
        .tecidos
        .get(app.venda_tecido_option)
        .map(|tecido| tecido.nome.as_str())
        .unwrap_or("Selecione");
    let vinculo_nome = vinculo
        .map(|vinculo| vinculo.cor_nome.as_str())
        .unwrap_or("Selecione");
    lines.push(format_select(
        VendaField::Tecido,
        field,
        "Tecido",
        tecido_nome,
    ));
    if app.venda_dropdown == Some(VendaField::Tecido) {
        push_tecido_options(&mut lines, &app.tecidos, app.venda_tecido_option);
    }
    lines.push(format_select(
        VendaField::Vinculo,
        field,
        "Vinculo",
        vinculo_nome,
    ));
    if app.venda_dropdown == Some(VendaField::Vinculo) {
        push_vinculo_options(&mut lines, &app.venda_vinculos, app.venda_vinculo_option);
    }
    if let Some(vinculo) = vinculo {
        lines.push(Line::from(format!(
            "SKU: {}",
            vinculo.sku.as_deref().unwrap_or("sem-sku")
        )));
    }
    lines.push(Line::from(""));
    lines.push(format_select(
        VendaField::Preco,
        field,
        "Preco Unitario",
        &preco_option_label(app.venda_preco_option, preco),
    ));
    if app.venda_dropdown == Some(VendaField::Preco) {
        push_preco_options(&mut lines, vinculo, app.venda_preco_option, preco);
    }
    lines.extend([
        format_field(
            VendaField::Quantidade,
            field,
            if app.editing_venda_item.is_some() {
                "Atualizar"
            } else {
                "Lancar"
            },
            quantidade,
        ),
        Line::from(""),
        format_action(
            VendaField::Finalizar,
            field,
            if app.editing_venda_id.is_some() {
                "[Salvar]"
            } else {
                "[Finalizar]"
            },
            true,
        ),
        format_action(VendaField::Cancelar, field, "[Cancelar]", true),
    ]);
    if app.editing_venda_id.is_some() {
        lines.push(format_destructive_action(
            VendaField::Excluir,
            field,
            "[Excluir]",
        ));
    }
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title(if app.editing_venda_id.is_some() {
                "Vendas > Historico > Editar"
            } else {
                "Vendas > Nova Venda > Lancamento"
            })
            .borders(Borders::ALL),
    );
    frame.render_widget(widget, area);
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
        let is_selected = focused && index == selected;
        let marker = if is_selected { "> " } else { "  " };
        let editing_marker = if editing == Some(index) { " *" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(marker, selected_style(is_selected)),
            Span::raw(format!("{}{}", item.descricao, editing_marker)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::raw(format!(
                "QTD: {} x R${} - Total: R${}",
                format_quantity(item.quantidade),
                format_money(item.preco_unitario),
                format_money(item.total())
            )),
        ]));
        lines.push(Line::from(""));
    }
    let total_quantity = itens.iter().map(|item| item.quantidade).sum::<f64>();
    let total = itens.iter().map(VendaItem::total).sum::<f64>();
    lines.push(Line::from(format!(
        "QTD Total: {}",
        format_quantity(total_quantity)
    )));
    lines.push(Line::from(format!(
        "Total da Venda: R${}",
        format_money(total)
    )));
    let widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title(if focused {
                "Resumo da venda > Enter editar | Del excluir"
            } else {
                "Resumo da venda"
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

fn format_money(value: f64) -> String {
    format!("{value:.2}").replace('.', ",")
}

fn format_optional_money(value: Option<f64>) -> String {
    value
        .map(format_money)
        .unwrap_or_else(|| String::from("nao definido"))
}

fn format_quantity(value: f64) -> String {
    format_money(value)
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

fn format_select(
    field: VendaField,
    selected: VendaField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(format!("[{value}]"), Style::default().fg(Color::Yellow)),
    ])
}

fn push_tecido_options(lines: &mut Vec<Line<'static>>, tecidos: &[TecidoRecord], selected: usize) {
    for (index, tecido) in tecidos.iter().enumerate() {
        let is_current = index == selected;
        lines.push(Line::from(vec![
            Span::styled(
                if is_current { "  > " } else { "    " },
                selected_style(is_current),
            ),
            Span::styled(
                format!("[{}]", tecido.nome),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }
}

fn push_vinculo_options(
    lines: &mut Vec<Line<'static>>,
    vinculos: &[VinculoRecord],
    selected: usize,
) {
    if vinculos.is_empty() {
        lines.push(Line::from("    Nenhum vinculo cadastrado"));
        return;
    }
    for (index, vinculo) in vinculos.iter().enumerate() {
        let is_current = index == selected;
        lines.push(Line::from(vec![
            Span::styled(
                if is_current { "  > " } else { "    " },
                selected_style(is_current),
            ),
            Span::styled(
                format!("[{}]", vinculo.cor_nome),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }
}

fn push_preco_options(
    lines: &mut Vec<Line<'static>>,
    vinculo: Option<&VinculoRecord>,
    selected: usize,
    manual: &str,
) {
    for option in 0..3 {
        let is_current = option == selected;
        lines.push(Line::from(vec![
            Span::styled(
                if is_current { "  > " } else { "    " },
                selected_style(is_current),
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

fn format_action(
    field: VendaField,
    selected: VendaField,
    label: &str,
    enabled: bool,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let suffix = if enabled { "" } else { "  indisponivel" };
    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(label.to_string()),
        Span::raw(suffix),
    ])
}

fn render_finalizar_dialog(
    frame: &mut Frame,
    area: Rect,
    selected: FinalizarVendaOption,
    editing: bool,
) {
    let popup_area = centered_rect(54, 7, area);
    render_dialog_background(frame, popup_area);
    let action = if editing { "Salvar" } else { "Finalizar" };
    let action_print = if editing {
        "Salvar e Imprimir"
    } else {
        "Finalizar e Imprimir"
    };
    let line = Line::from(vec![
        Span::styled(
            if selected == FinalizarVendaOption::Finalizar {
                "> "
            } else {
                "  "
            },
            selected_style(selected == FinalizarVendaOption::Finalizar),
        ),
        Span::raw(format!("[{action}]   ")),
        Span::styled(
            if selected == FinalizarVendaOption::FinalizarEImprimir {
                "> "
            } else {
                "  "
            },
            selected_style(selected == FinalizarVendaOption::FinalizarEImprimir),
        ),
        Span::raw(format!("[{action_print}]")),
    ]);
    let dialog = Paragraph::new(Text::from(vec![
        Line::from(if editing {
            "Salvar alteracoes da venda?"
        } else {
            "Concluir venda?"
        }),
        Line::from(""),
        line,
    ]))
    .block(
        Block::default()
            .title(if editing {
                "Salvar venda"
            } else {
                "Finalizar venda"
            })
            .borders(Borders::ALL)
            .style(Style::default().bg(DIALOG_BG))
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .style(Style::default().bg(DIALOG_BG))
    .alignment(Alignment::Center);

    frame.render_widget(dialog, popup_area);
}

fn format_destructive_action(
    field: VendaField,
    selected: VendaField,
    label: &str,
) -> Line<'static> {
    destructive_action_line(field, selected, label)
}
