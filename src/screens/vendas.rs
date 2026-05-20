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
        DIALOG_BG, SIDE_PANEL_WIDTH, centered_rect, color_swatch, list_state_with_lookahead,
        render_dialog_background, selected_style,
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
        render_excluir_dialog(frame, area);
    }
    if app.pending_delete_venda_item {
        render_excluir_item_dialog(frame, area);
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
        vendas
            .iter()
            .map(|venda| {
                ListItem::new(format!(
                    "#{}  {}  {} itens  Total R${}",
                    venda.id,
                    venda.created_at,
                    venda.itens,
                    format_money(venda.total)
                ))
            })
            .collect()
    };
    let mut state = list_state_with_lookahead(
        (!vendas.is_empty() && selected_field == 2).then_some(selected),
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
    let items = tecidos
        .iter()
        .enumerate()
        .map(|(index, tecido)| ListItem::new(format!("{}. {}", index + 1, tecido.nome)));
    let mut state = list_state_with_lookahead(Some(selected), tecidos.len(), area);
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
    lines.extend([
        format_field(VendaField::Preco, field, "Preco Unitario", preco),
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
        lines.push(format_action(VendaField::Excluir, field, "[Excluir]", true));
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
    let total = itens.iter().map(VendaItem::total).sum::<f64>();
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

fn render_excluir_dialog(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(48, 7, area);
    render_dialog_background(frame, popup_area);
    let dialog = Paragraph::new(Text::from(vec![
        Line::from("Excluir esta venda?"),
        Line::from(""),
        Line::from("[Enter/S] Confirmar   [Esc/N] Voltar"),
    ]))
    .block(
        Block::default()
            .title("Confirmar exclusao")
            .borders(Borders::ALL)
            .style(Style::default().bg(DIALOG_BG))
            .border_style(Style::default().fg(Color::Red)),
    )
    .style(Style::default().bg(DIALOG_BG))
    .alignment(Alignment::Center);

    frame.render_widget(dialog, popup_area);
}

fn render_excluir_item_dialog(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(48, 7, area);
    render_dialog_background(frame, popup_area);
    let dialog = Paragraph::new(Text::from(vec![
        Line::from("Excluir este lancamento?"),
        Line::from(""),
        Line::from("[Enter/S] Confirmar   [Esc/N] Voltar"),
    ]))
    .block(
        Block::default()
            .title("Confirmar exclusao")
            .borders(Borders::ALL)
            .style(Style::default().bg(DIALOG_BG))
            .border_style(Style::default().fg(Color::Red)),
    )
    .style(Style::default().bg(DIALOG_BG))
    .alignment(Alignment::Center);

    frame.render_widget(dialog, popup_area);
}
