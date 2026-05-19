mod forms;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use ratatui_image::Image as TuiImage;

use crate::{
    app::App,
    db::{CorRecord, EstampaRecord, TecidoRecord, VinculoRecord},
    models::*,
    ui::{centered_rect, color_swatch, render_dialog_background},
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    match app.dados_screen {
        DadosScreen::Menu => render_menu(frame, area, app.dados_option),
        DadosScreen::Tecidos => render_tecidos(frame, area, app.tecido_option, &app.tecidos),
        DadosScreen::CadastrarTecido => forms::render_cadastrar_tecido(
            frame,
            area,
            &app.tecido_form,
            &app.tecidos,
            app.editing_tecido_id,
            app.pending_delete,
            app.tecido_select_dropdown,
        ),
        DadosScreen::Cores => render_cores(
            frame,
            area,
            app.cor_option,
            &app.cores,
            app.color_delta_e_threshold,
        ),
        DadosScreen::CadastrarCor => forms::render_cadastrar_cor(
            frame,
            area,
            &app.cor_form,
            &app.cores,
            app.editing_cor_id,
            app.color_delta_e_threshold,
            app.pending_delete,
        ),
        DadosScreen::Estampas => render_estampas(frame, area, app.cor_option, &app.estampas),
        DadosScreen::CadastrarEstampa => forms::render_cadastrar_estampa(
            frame,
            area,
            &app.estampa_form,
            &app.estampas,
            app.editing_estampa_id,
            app.pending_delete,
        ),
        DadosScreen::VinculosMenu => render_vinculos_menu(frame, area, app.vinculo_menu_option),
        DadosScreen::VinculosSelecionarTecidoCriar => render_vinculo_tecidos(
            frame,
            area,
            "Dados > Vinculos > Criar > Selecione o tecido",
            app.vinculo_tecido_option,
            &app.tecidos,
        ),
        DadosScreen::VinculosSelecionarTecidoVer => render_vinculo_tecidos(
            frame,
            area,
            "Dados > Vinculos > Ver > Selecione o tecido",
            app.vinculo_tecido_option,
            &app.tecidos,
        ),
        DadosScreen::VinculosSelecionarCores => render_vinculo_cores(
            frame,
            area,
            app.vinculo_criar_option,
            if app
                .tecidos
                .get(app.vinculo_tecido_option)
                .map(|tecido| tecido.tipo == "Estampado")
                .unwrap_or(false)
            {
                VinculoItems::Estampas(&app.estampas)
            } else {
                VinculoItems::Cores(&app.cores)
            },
            &app.selected_vinculo_cores,
        ),
        DadosScreen::VinculosLista => {
            render_vinculos_lista(frame, area, app.vinculo_lista_option, &app.vinculos)
        }
        DadosScreen::VinculoDetalhe => render_vinculo_detalhe(frame, area, app),
        DadosScreen::ListaPrecosMenu => {
            render_lista_precos_menu(frame, area, app.lista_precos_option)
        }
        DadosScreen::ListaPrecosAtacado | DadosScreen::ListaPrecosVarejo => {
            render_lista_precos(frame, area, app)
        }
        DadosScreen::ListaPrecosTecido => render_lista_precos_tecido(frame, area, app),
        DadosScreen::ListaPrecosVinculos => render_lista_precos_vinculos(frame, area, app),
    }
}

enum VinculoItems<'a> {
    Cores(&'a [CorRecord]),
    Estampas(&'a [EstampaRecord]),
}

fn render_menu(frame: &mut Frame, area: Rect, selected: DadosOption) {
    let items = DadosOption::ALL
        .iter()
        .enumerate()
        .map(|(index, option)| ListItem::new(format!("{}. {}", index + 1, option.title())));
    let mut state = ListState::default().with_selected(Some(selected.index()));
    let list = List::new(items)
        .block(Block::default().title("Dados").borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_tecidos(frame: &mut Frame, area: Rect, selected: usize, tecidos: &[TecidoRecord]) {
    let items = std::iter::once(String::from("[Cadastrar tecido]"))
        .chain(tecidos.iter().map(|tecido| tecido.nome.clone()))
        .enumerate()
        .map(|(index, tecido)| ListItem::new(format!("{}. {}", index + 1, tecido)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Tecido")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_lista_precos_menu(frame: &mut Frame, area: Rect, selected: usize) {
    let items = ["Atacado", "Varejo"]
        .iter()
        .enumerate()
        .map(|(index, option)| ListItem::new(format!("{}. {}", index + 1, option)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Lista de Precos")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_lista_precos(frame: &mut Frame, area: Rect, app: &App) {
    let items = if app.tecidos.is_empty() {
        vec![ListItem::new("Nenhum tecido cadastrado.")]
    } else {
        app.tecidos
            .iter()
            .enumerate()
            .map(|tecido| {
                let (index, tecido) = tecido;
                let price = tecido_lista_preco(tecido, app.lista_precos_tipo);
                let overrides = tecido_lista_override_count(tecido, app.lista_precos_tipo);
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{}. ", index + 1)),
                    Span::styled(
                        format!("{:<4}", tecido.sku),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(format!(
                        " {} | custo R$ {} | preco R$ ",
                        tecido.nome,
                        format_optional_money(tecido.custo_base)
                    )),
                    Span::styled(
                        format_optional_money(price),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(format!(" | overrides {overrides}")),
                ]))
            })
            .collect()
    };
    let mut state = ListState::default().with_selected(Some(app.lista_precos_tecido_option));
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "Dados > Lista de Precos > {}",
                    app.lista_precos_tipo.title()
                ))
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_lista_precos_tecido(frame: &mut Frame, area: Rect, app: &App) {
    let Some(tecido) = app.tecidos.get(app.lista_precos_tecido_option) else {
        return;
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(44), Constraint::Length(44)])
        .split(area);
    let current_price = tecido_lista_preco(tecido, app.lista_precos_tipo);
    let value = if app.editing_lista_preco_tecido {
        if app.lista_precos_tecido_input.is_empty() {
            String::from("_")
        } else {
            app.lista_precos_tecido_input.clone()
        }
    } else {
        format_optional_money(current_price)
    };
    let items = [
        ListItem::new(format!(
            "Preco {} do tecido: R$ {}",
            app.lista_precos_tipo.title(),
            value
        )),
        ListItem::new("[Vinculos / Excecoes]"),
        ListItem::new("[Voltar]"),
    ];
    let mut state = ListState::default().with_selected(Some(app.lista_precos_tecido_detail_option));
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "Dados > Lista de Precos > {} > {}",
                    app.lista_precos_tipo.title(),
                    tecido.nome
                ))
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, chunks[0], &mut state);

    let overrides = tecido_lista_override_count(tecido, app.lista_precos_tipo);
    let summary = vec![
        Line::from(format!("SKU: {}", tecido.sku)),
        Line::from(format!(
            "Custo base: R$ {}",
            format_optional_money(tecido.custo_base)
        )),
        Line::from(format!(
            "Preco {}: R$ {}",
            app.lista_precos_tipo.title(),
            format_optional_money(current_price)
        )),
        Line::from(format!("Vinculos especificos: {overrides}")),
    ];
    frame.render_widget(
        Paragraph::new(summary).block(Block::default().title("Resumo").borders(Borders::ALL)),
        chunks[1],
    );
}

fn render_lista_precos_vinculos(frame: &mut Frame, area: Rect, app: &App) {
    let items = if app.vinculos.is_empty() {
        vec![ListItem::new("Nenhum vinculo cadastrado para este tecido.")]
    } else {
        app.vinculos
            .iter()
            .enumerate()
            .map(|(index, vinculo)| {
                let effective = vinculo_lista_preco_efetivo(vinculo, app.lista_precos_tipo);
                let base = vinculo_lista_preco_base(vinculo, app.lista_precos_tipo);
                let override_price = vinculo_lista_preco_override(vinculo, app.lista_precos_tipo);
                let origin = if override_price.is_some() {
                    "especifico"
                } else {
                    "base"
                };
                let value = if app.editing_lista_preco_vinculo
                    && index == app.lista_precos_vinculo_option
                {
                    if app.lista_precos_vinculo_input.is_empty() {
                        String::from("_")
                    } else {
                        app.lista_precos_vinculo_input.clone()
                    }
                } else {
                    format_optional_money(effective)
                };
                ListItem::new(format!(
                    "{}. {} | R$ {} | {} | base R$ {}",
                    index + 1,
                    vinculo.cor_nome,
                    value,
                    origin,
                    format_optional_money(base)
                ))
            })
            .collect()
    };
    let mut state = ListState::default().with_selected(Some(app.lista_precos_vinculo_option));
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "Dados > Lista de Precos > {} > Vinculos",
                    app.lista_precos_tipo.title()
                ))
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_cores(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    cores: &[CorRecord],
    delta_e_threshold: f64,
) {
    let conflicts = closest_color_conflicts(cores, delta_e_threshold);
    let items = std::iter::once(ListItem::new("1. [Cadastrar Cor]")).chain(
        cores.iter().enumerate().map(|(index, cor)| {
            let hex = cor.codigo_hex.as_deref().unwrap_or("#");
            let sku = cor.sku.as_deref().unwrap_or("____-__");
            let conflict = conflicts.get(index).and_then(Option::as_ref);
            let mut spans = vec![
                Span::raw(format!("{}. {} - ", index + 2, sku)),
                color_swatch(hex),
                Span::raw(format!(" {} ({hex})", cor.nome)),
            ];
            if let Some(conflict) = conflict {
                spans.push(Span::styled(
                    format!(
                        " * Conflito: {} - {} - Delta E {:.2}",
                        conflict.nome, conflict.hex, conflict.delta_e
                    ),
                    Style::default().fg(Color::Red),
                ));
            }
            ListItem::new(Line::from(spans))
        }),
    );
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Cores")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_estampas(frame: &mut Frame, area: Rect, selected: usize, estampas: &[EstampaRecord]) {
    let items = std::iter::once(ListItem::new("1. [Cadastrar Estampa]")).chain(
        estampas.iter().enumerate().map(|(index, estampa)| {
            let sku = estampa.sku.as_deref().unwrap_or("____-__");
            ListItem::new(format!("{}. {} - {}", index + 2, sku, estampa.nome))
        }),
    );
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Estampas")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculos_menu(frame: &mut Frame, area: Rect, selected: usize) {
    let items = ["[Criar Vinculos]", "[Ver Vinculos]"]
        .iter()
        .enumerate()
        .map(|(index, item)| ListItem::new(format!("{}. {}", index + 1, item)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Vinculos")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculo_tecidos(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    selected: usize,
    tecidos: &[TecidoRecord],
) {
    let items = tecidos
        .iter()
        .enumerate()
        .map(|(index, tecido)| ListItem::new(format!("{}. {}", index + 1, tecido.nome)));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculo_cores(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    items_source: VinculoItems<'_>,
    selected_cores: &[i64],
) {
    let title = match items_source {
        VinculoItems::Cores(_) => "Dados > Vinculos > Criar > Selecione as cores",
        VinculoItems::Estampas(_) => "Dados > Vinculos > Criar > Selecione as estampas",
    };
    let mut items = Vec::new();
    match items_source {
        VinculoItems::Cores(cores) => {
            for (index, cor) in cores.iter().enumerate() {
                let marker = if selected_cores.contains(&cor.id) {
                    "[+]"
                } else {
                    "[ ]"
                };
                let sku = cor.sku.as_deref().unwrap_or("____-__");
                let hex = cor.codigo_hex.as_deref().unwrap_or("#");
                items.push(ListItem::new(Line::from(vec![
                    Span::raw(format!("{}. {} {} - ", index + 1, marker, sku)),
                    color_swatch(hex),
                    Span::raw(format!(" {}", cor.nome)),
                ])));
            }
        }
        VinculoItems::Estampas(estampas) => {
            for (index, estampa) in estampas.iter().enumerate() {
                let marker = if selected_cores.contains(&estampa.id) {
                    "[+]"
                } else {
                    "[ ]"
                };
                let sku = estampa.sku.as_deref().unwrap_or("____-__");
                items.push(ListItem::new(format!(
                    "{}. {} {} - {}",
                    index + 1,
                    marker,
                    sku,
                    estampa.nome
                )));
            }
        }
    }
    items.push(ListItem::new("[Confirmar]"));
    items.push(ListItem::new("[Voltar]"));
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculos_lista(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    vinculos: &[VinculoRecord],
) {
    let items = vinculos.iter().enumerate().map(|(index, vinculo)| {
        let sku = vinculo.sku.as_deref().unwrap_or("sem-sku");
        let hex = vinculo.cor_hex.as_deref().unwrap_or("#");
        let image_count = App::vinculo_record_image_count(vinculo);
        ListItem::new(Line::from(vec![
            Span::raw(format!(
                "{}. [{}/4] {} - {} / ",
                index + 1,
                image_count,
                sku,
                vinculo.tecido_nome
            )),
            color_swatch(hex),
            Span::raw(format!(" {}", vinculo.cor_nome)),
        ]))
    });
    let mut state =
        ListState::default().with_selected(Some(selected.min(vinculos.len().saturating_sub(1))));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Dados > Vinculos > Cores vinculadas")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().bg(Color::Cyan).bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_vinculo_detalhe(frame: &mut Frame, area: Rect, app: &App) {
    let Some(vinculo) = app.vinculos.get(app.vinculo_lista_option) else {
        return;
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(54), Constraint::Length(46)])
        .split(area);

    let slot_items = VinculoDetalheOption::ALL.iter().map(|option| match option {
        VinculoDetalheOption::Slot(slot) => {
            let has_image = match slot {
                VinculoImageSlot::Original => app.vinculo_images.imagem_original.is_some(),
                VinculoImageSlot::Brand => app.vinculo_images.imagem_brand.is_some(),
                VinculoImageSlot::Modelo => app.vinculo_images.imagem_modelo.is_some(),
                VinculoImageSlot::Alternativa => app.vinculo_images.imagem_alternativa.is_some(),
            };
            let marker = if has_image { "[+]" } else { "[ ]" };
            ListItem::new(format!("{} {marker} {}", slot.index() + 1, slot.title()))
        }
        VinculoDetalheOption::Custo => {
            let effective = format_optional_money(vinculo.custo_efetivo);
            let base = format_optional_money(vinculo.tecido_custo_base);
            let value = if app.editing_vinculo_custo {
                if app.vinculo_custo_input.is_empty() {
                    String::from("_")
                } else {
                    app.vinculo_custo_input.clone()
                }
            } else {
                effective
            };
            let source = if vinculo.custo_override.is_some() {
                "especifico"
            } else {
                "base"
            };
            ListItem::new(format!(
                "[Custo vinculo] R$ {value} ({source}; base R$ {base})"
            ))
        }
        VinculoDetalheOption::Desfazer => ListItem::new("[Desfazer Vinculo]"),
    });
    let current_count = app.vinculo_current_image_count();
    let mut state = ListState::default().with_selected(Some(app.vinculo_detalhe_option.index()));
    let detail = List::new(slot_items)
        .block(
            Block::default()
                .title(format!(
                    "Vinculo {}/{} > {} / {} > Imagens {}/4",
                    app.vinculo_lista_option + 1,
                    app.vinculos.len(),
                    vinculo.tecido_nome,
                    vinculo.cor_nome,
                    current_count
                ))
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_stateful_widget(detail, chunks[0], &mut state);

    let preview = chunks[1];
    frame.render_widget(
        Block::default()
            .title(format!("Thumbnail {}", app.vinculo_image_slot.title()))
            .borders(Borders::ALL),
        preview,
    );
    let inner = preview.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    if let Some(started_at) = app.vinculo_image_upload_started {
        let spinner = ["|", "/", "-", "\\"][(started_at.elapsed().as_millis() as usize / 250) % 4];
        frame.render_widget(
            Paragraph::new(format!(
                "{spinner} Salvando imagem...\n\nAguarde o upload terminar para continuar."
            ))
            .style(Style::default().fg(Color::Yellow)),
            inner,
        );
    } else if let Some(protocol) = &app.vinculo_thumbnail {
        frame.render_widget(TuiImage::new(protocol).allow_clipping(true), inner);
    } else {
        let has_selected_image = match app.vinculo_image_slot {
            VinculoImageSlot::Original => app.vinculo_images.imagem_original.is_some(),
            VinculoImageSlot::Brand => app.vinculo_images.imagem_brand.is_some(),
            VinculoImageSlot::Modelo => app.vinculo_images.imagem_modelo.is_some(),
            VinculoImageSlot::Alternativa => app.vinculo_images.imagem_alternativa.is_some(),
        };
        let text = if has_selected_image {
            "Imagem salva, mas sem preview neste terminal."
        } else {
            "Sem imagem neste slot."
        };
        frame.render_widget(Paragraph::new(text), inner);
    }

    frame.render_widget(
        Paragraph::new(format!(
            "1-4 slot | Enter upload/editar/confirmar | Custo vazio usa base | Tab proximo vinculo | Shift+Tab anterior | {}",
            app.image_protocol_status
        ))
        .style(Style::default().fg(Color::DarkGray)),
        Rect {
            x: chunks[0].x.saturating_add(2),
            y: chunks[0].y + chunks[0].height.saturating_sub(2),
            width: chunks[0].width.saturating_sub(4),
            height: 1,
        },
    );

    if app.pending_unlink_vinculo {
        let dialog_area = centered_rect(56, 7, area);
        render_dialog_background(frame, dialog_area);
        let dialog = Paragraph::new(format!(
            "Desfazer vinculo de {} / {}?\n\nEle deixara de aparecer para novos lancamentos. Historico e imagens permanecem no banco.\nS/Enter confirma   N/Esc cancela",
            vinculo.tecido_nome, vinculo.cor_nome
        ))
        .block(Block::default().title("Confirmar").borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow));
        frame.render_widget(dialog, dialog_area);
    }
}

fn format_optional_money(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}").replace('.', ","))
        .unwrap_or_else(|| String::from("nao definido"))
}

fn tecido_lista_preco(tecido: &TecidoRecord, tipo: ListaPrecoTipo) -> Option<f64> {
    match tipo {
        ListaPrecoTipo::Atacado => tecido.preco_atacado,
        ListaPrecoTipo::Varejo => tecido.preco_varejo,
    }
}

fn tecido_lista_override_count(tecido: &TecidoRecord, tipo: ListaPrecoTipo) -> i64 {
    match tipo {
        ListaPrecoTipo::Atacado => tecido.preco_atacado_override_count,
        ListaPrecoTipo::Varejo => tecido.preco_varejo_override_count,
    }
}

fn vinculo_lista_preco_override(vinculo: &VinculoRecord, tipo: ListaPrecoTipo) -> Option<f64> {
    match tipo {
        ListaPrecoTipo::Atacado => vinculo.preco_atacado_override,
        ListaPrecoTipo::Varejo => vinculo.preco_varejo_override,
    }
}

fn vinculo_lista_preco_base(vinculo: &VinculoRecord, tipo: ListaPrecoTipo) -> Option<f64> {
    match tipo {
        ListaPrecoTipo::Atacado => vinculo.tecido_preco_atacado,
        ListaPrecoTipo::Varejo => vinculo.tecido_preco_varejo,
    }
}

fn vinculo_lista_preco_efetivo(vinculo: &VinculoRecord, tipo: ListaPrecoTipo) -> Option<f64> {
    match tipo {
        ListaPrecoTipo::Atacado => vinculo.preco_atacado_efetivo,
        ListaPrecoTipo::Varejo => vinculo.preco_varejo_efetivo,
    }
}
