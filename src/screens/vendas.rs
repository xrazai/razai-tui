use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    app::App,
    db::{TecidoRecord, VinculoRecord},
    models::{FinalizarVendaOption, VendaField, VendaItem, VendasScreen},
    ui::{centered_rect, color_swatch, selected_style},
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
            app,
            app.venda_field,
            &app.venda_preco,
            &app.venda_quantidade,
            app.venda_vinculos.get(app.venda_vinculo_option),
        ),
        VendasScreen::Historico => {
            render_historico(
                frame,
                chunks[0],
                &app.vendas_historico,
                app.venda_historico_option,
            );
        }
    }

    render_resumo(frame, chunks[1], &app.venda_itens);
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
}

fn render_historico(
    frame: &mut Frame,
    area: Rect,
    vendas: &[crate::db::VendaHistoricoRecord],
    selected: usize,
) {
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
    let mut state = ListState::default().with_selected((!vendas.is_empty()).then_some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Vendas > Historico")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
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
        format_field(VendaField::Quantidade, field, "Lancar", quantidade),
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

fn render_resumo(frame: &mut Frame, area: Rect, itens: &[VendaItem]) {
    let mut lines = Vec::new();
    for item in itens {
        lines.push(Line::from(item.descricao.clone()));
        lines.push(Line::from(format!(
            "QTD: {} x R${} - Total: R${}",
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
            .title("Resumo do pedido")
            .borders(Borders::ALL),
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
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .alignment(Alignment::Center);

    frame.render_widget(dialog, popup_area);
}

fn render_excluir_dialog(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(48, 7, area);
    let dialog = Paragraph::new(Text::from(vec![
        Line::from("Excluir esta venda?"),
        Line::from(""),
        Line::from("[Enter/S] Confirmar   [Esc/N] Voltar"),
    ]))
    .block(
        Block::default()
            .title("Confirmar exclusao")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    )
    .alignment(Alignment::Center);

    frame.render_widget(dialog, popup_area);
}
