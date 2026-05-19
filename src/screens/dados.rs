mod forms;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::{
    app::App,
    db::{CorRecord, EstampaRecord, TecidoRecord, VinculoRecord},
    models::*,
    ui::color_swatch,
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

fn render_cores(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    cores: &[CorRecord],
    delta_e_threshold: f64,
) {
    let items = std::iter::once(ListItem::new("1. [Cadastrar Cor]")).chain(
        cores.iter().enumerate().map(|(index, cor)| {
            let hex = cor.codigo_hex.as_deref().unwrap_or("#");
            let sku = cor.sku.as_deref().unwrap_or("____-__");
            let conflict = nearby_colors(hex, cores, Some(cor.id), delta_e_threshold)
                .into_iter()
                .next();
            let mut spans = vec![
                Span::raw(format!("{}. {} - ", index + 2, sku)),
                color_swatch(hex),
                Span::raw(format!(" {} ({hex})", cor.nome)),
            ];
            if let Some(conflict) = conflict {
                spans.push(Span::styled(
                    format!(
                        " * Conflito mais proximo: {} (Delta E {:.2})",
                        conflict.nome, conflict.delta_e
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
        ListItem::new(Line::from(vec![
            Span::raw(format!(
                "{}. {} - {} / ",
                index + 1,
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
