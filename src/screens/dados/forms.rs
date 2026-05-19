use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    db::{CorRecord, EstampaRecord, TecidoRecord},
    models::*,
    ui::{SIDE_PANEL_WIDTH, centered_rect, color_swatch, selected_style},
};

pub(super) fn render_cadastrar_cor(
    frame: &mut Frame,
    area: Rect,
    form: &CorForm,
    cores: &[CorRecord],
    editing_id: Option<i64>,
    delta_e_threshold: f64,
    pending_delete: bool,
) {
    let hex_complete = is_complete_hex_color(&form.hex);
    let nearby = if hex_complete {
        nearby_colors(&form.hex, cores, editing_id, delta_e_threshold)
    } else {
        Vec::new()
    };
    let lines = vec![
        format_cor_hex_field(CorField::Hex, form.selected_field, "Hex", &form.hex),
        format_cor_field(
            CorField::Nome,
            form.selected_field,
            "Nome da Cor",
            &form.nome,
        ),
        Line::from(""),
        format_cor_action(
            CorField::Confirmar,
            form.selected_field,
            "[Confirmar]",
            form.is_valid() && nearby.is_empty(),
        ),
        format_cor_action(CorField::Voltar, form.selected_field, "[Voltar]", true),
        if editing_id.is_some() {
            format_cor_action(CorField::Excluir, form.selected_field, "[Excluir]", true)
        } else {
            Line::from("")
        },
    ];
    let form_widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Dados > Cores > Cadastrar Cor")
            .borders(Borders::ALL),
    );
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(SIDE_PANEL_WIDTH)])
        .split(area);
    let proximity_lines = if hex_complete {
        let mut lines = vec![Line::from(vec![
            Span::raw("Limiar Delta E: "),
            Span::styled(
                format!("{delta_e_threshold:.2}"),
                Style::default().fg(Color::Yellow),
            ),
        ])];
        if nearby.is_empty() {
            lines.push(Line::from("Nenhuma cor proxima abaixo do limiar."));
        } else {
            lines.push(Line::from(Span::styled(
                "Cor bloqueada: proximidade abaixo do limiar.",
                Style::default().fg(Color::Red),
            )));
            for color in nearby.iter().take(6) {
                lines.push(Line::from(vec![
                    color_swatch(&color.hex),
                    Span::raw(format!(
                        " {} ({}) {} Delta E {:.2}",
                        color.nome,
                        color.sku.as_deref().unwrap_or("sem SKU"),
                        color.hex,
                        color.delta_e
                    )),
                ]));
            }
        }
        lines
    } else {
        Vec::new()
    };
    let sku = Paragraph::new(Text::from(vec![
        Line::from(form.sku(cores, editing_id)),
        Line::from(""),
    ]
    .into_iter()
    .chain(proximity_lines)
    .collect::<Vec<_>>()))
        .block(Block::default().title("SKU").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    frame.render_widget(form_widget, chunks[0]);
    frame.render_widget(sku, chunks[1]);

    if pending_delete {
        render_confirm_dialog(frame, area, "Excluir esta cor?");
    }
}

pub(super) fn render_cadastrar_estampa(
    frame: &mut Frame,
    area: Rect,
    form: &EstampaForm,
    estampas: &[EstampaRecord],
    editing_id: Option<i64>,
    pending_delete: bool,
) {
    let lines = vec![
        format_estampa_field(
            EstampaField::Nome,
            form.selected_field,
            "Nome da Estampa",
            &form.nome,
        ),
        Line::from(""),
        format_estampa_action(
            EstampaField::Confirmar,
            form.selected_field,
            "[Confirmar]",
            form.is_valid(),
        ),
        format_estampa_action(EstampaField::Voltar, form.selected_field, "[Voltar]", true),
        if editing_id.is_some() {
            format_estampa_action(
                EstampaField::Excluir,
                form.selected_field,
                "[Excluir]",
                true,
            )
        } else {
            Line::from("")
        },
    ];
    let form_widget = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("Dados > Estampas > Cadastrar Estampa")
            .borders(Borders::ALL),
    );
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(SIDE_PANEL_WIDTH)])
        .split(area);
    let sku = Paragraph::new(form.sku(estampas, editing_id))
        .block(Block::default().title("SKU").borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(form_widget, chunks[0]);
    frame.render_widget(sku, chunks[1]);

    if pending_delete {
        render_confirm_dialog(frame, area, "Excluir esta estampa?");
    }
}

pub(super) fn render_cadastrar_tecido(
    frame: &mut Frame,
    area: Rect,
    form: &TecidoForm,
    tecidos: &[TecidoRecord],
    editing_tecido_id: Option<i64>,
    pending_delete: bool,
    dropdown: Option<TecidoField>,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(SIDE_PANEL_WIDTH)])
        .split(area);
    let calculated = form.calculated_values();
    let mut fields = vec![
        format_field(
            TecidoField::Nome,
            form.selected_field,
            "Nome *",
            &form.nome,
            None,
        ),
        format_field(
            TecidoField::Composicao,
            form.selected_field,
            "Composicao *",
            &form.composicao,
            None,
        ),
        format_field(
            TecidoField::Largura,
            form.selected_field,
            "Largura *",
            &form.largura,
            None,
        ),
    ];
    push_select(
        &mut fields,
        TecidoField::Tipo,
        form.selected_field,
        "Tipo",
        form.tipo.value(TIPO_OPTIONS),
        TIPO_OPTIONS,
        dropdown,
    );
    push_select(
        &mut fields,
        TecidoField::Transparencia,
        form.selected_field,
        "Transparencia",
        form.transparencia.value(NIVEL_OPTIONS),
        NIVEL_OPTIONS,
        dropdown,
    );
    push_select(
        &mut fields,
        TecidoField::Elasticidade,
        form.selected_field,
        "Elasticidade",
        form.elasticidade.value(NIVEL_OPTIONS),
        NIVEL_OPTIONS,
        dropdown,
    );
    push_select(
        &mut fields,
        TecidoField::Acabamento,
        form.selected_field,
        "Acabamento",
        form.acabamento.value(ACABAMENTO_OPTIONS),
        ACABAMENTO_OPTIONS,
        dropdown,
    );
    fields.extend([
        format_field(
            TecidoField::Rendimento,
            form.selected_field,
            "Rendimento m/kg",
            &form.rendimento,
            calculated.rendimento,
        ),
        format_field(
            TecidoField::GramaturaLinear,
            form.selected_field,
            "Gramatura Linear g/m",
            &form.gramatura_linear,
            calculated.gramatura_linear,
        ),
        format_field(
            TecidoField::GramaturaM2,
            form.selected_field,
            "Gramatura g/m2",
            &form.gramatura_m2,
            calculated.gramatura_m2,
        ),
        Line::from(""),
        format_submit(
            form.selected_field,
            form.is_valid(),
            editing_tecido_id.is_some(),
        ),
        format_tecido_action(TecidoField::Voltar, form.selected_field, "[Voltar]"),
        format_delete(form.selected_field, editing_tecido_id.is_some()),
    ]);
    let form_widget = Paragraph::new(Text::from(fields))
        .block(
            Block::default()
                .title("Dados > Tecido > Cadastrar tecido")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left);
    let sku = Paragraph::new(form.sku(tecidos, editing_tecido_id))
        .block(Block::default().title("SKU").borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(form_widget, chunks[0]);
    frame.render_widget(sku, chunks[1]);

    if pending_delete {
        render_confirm_dialog(frame, area, "Excluir este tecido?");
    }
}

pub(super) fn format_field(
    field: TecidoField,
    selected: TecidoField,
    label: &str,
    value: &str,
    calculated: Option<f64>,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let value = if value.is_empty() { "_" } else { value };
    let mut spans = vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
    ];

    if let Some(number) = calculated {
        let calculated = match field {
            TecidoField::GramaturaLinear | TecidoField::GramaturaM2 => {
                format!("  calculado: {}", round_to_nearest_ten(number))
            }
            _ => format!("  calculado: {:.2}", number),
        };
        spans.push(Span::raw(calculated));
    }

    Line::from(spans)
}

pub(super) fn format_select(
    field: TecidoField,
    selected: TecidoField,
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

fn push_select(
    lines: &mut Vec<Line<'static>>,
    field: TecidoField,
    selected: TecidoField,
    label: &str,
    value: &str,
    options: &'static [&'static str],
    dropdown: Option<TecidoField>,
) {
    lines.push(format_select(field, selected, label, value));
    if dropdown != Some(field) {
        return;
    }

    for option in options {
        let is_current = *option == value;
        lines.push(Line::from(vec![
            Span::styled(
                if is_current { "  > " } else { "    " },
                selected_style(is_current),
            ),
            Span::styled(format!("[{option}]"), Style::default().fg(Color::Yellow)),
        ]));
    }
}

pub(super) fn format_cor_field(
    field: CorField,
    selected: CorField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let value = if value.is_empty() {
        if field == CorField::Hex { "#" } else { "_" }
    } else {
        value
    };

    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
    ])
}

pub(super) fn format_cor_hex_field(
    field: CorField,
    selected: CorField,
    label: &str,
    value: &str,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let value = if value.is_empty() { "#" } else { value };

    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(format!("{label}: ")),
        Span::styled(value.to_string(), Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        color_swatch(value),
        if parse_hex_color(value).is_some() {
            Span::raw("")
        } else {
            Span::raw("  hex invalido, use #RRGGBB")
        },
    ])
}

pub(super) fn format_cor_action(
    field: CorField,
    selected: CorField,
    label: &str,
    enabled: bool,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let suffix = if enabled {
        ""
    } else {
        "  campos obrigatorios pendentes"
    };

    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(label.to_string()),
        Span::raw(suffix),
    ])
}

pub(super) fn format_estampa_field(
    field: EstampaField,
    selected: EstampaField,
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

pub(super) fn format_estampa_action(
    field: EstampaField,
    selected: EstampaField,
    label: &str,
    enabled: bool,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    let suffix = if enabled {
        ""
    } else {
        "  campos obrigatorios pendentes"
    };

    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(label.to_string()),
        Span::raw(suffix),
    ])
}

pub(super) fn format_submit(selected: TecidoField, valid: bool, editing: bool) -> Line<'static> {
    let marker = if selected == TecidoField::Salvar {
        ">"
    } else {
        " "
    };
    let status = if valid {
        ""
    } else {
        "  campos obrigatorios pendentes"
    };

    Line::from(vec![
        Span::styled(
            format!("{marker} "),
            selected_style(selected == TecidoField::Salvar),
        ),
        Span::raw(if editing {
            "[Confirmar]"
        } else {
            "[Confirmar]"
        }),
        Span::raw(status),
    ])
}

pub(super) fn format_delete(selected: TecidoField, editing: bool) -> Line<'static> {
    if !editing {
        return Line::from("");
    }

    let marker = if selected == TecidoField::Excluir {
        ">"
    } else {
        " "
    };

    Line::from(vec![
        Span::styled(
            format!("{marker} "),
            selected_style(selected == TecidoField::Excluir),
        ),
        Span::raw("[Excluir]"),
    ])
}

pub(super) fn format_tecido_action(
    field: TecidoField,
    selected: TecidoField,
    label: &str,
) -> Line<'static> {
    let marker = if field == selected { ">" } else { " " };
    Line::from(vec![
        Span::styled(format!("{marker} "), selected_style(field == selected)),
        Span::raw(label.to_string()),
    ])
}

pub(super) fn render_confirm_dialog(frame: &mut Frame, area: Rect, message: &str) {
    let popup_area = centered_rect(54, 7, area);
    let dialog = Paragraph::new(format!("{message}\n\nS = confirmar   N/Esc = cancelar"))
        .block(
            Block::default()
                .title("Confirmacao destrutiva")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(dialog, popup_area);
}
