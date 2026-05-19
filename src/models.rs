mod sku;
pub use sku::{build_estampa_vinculo_sku, build_vinculo_sku};

use crate::db::{CorRecord, EstampaRecord, TecidoRecord};

mod core;
pub use core::{AgentAction, AgentDraft, ChatMessage, ChatState, Focus, PedidosScreen};

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum VendasScreen {
    #[default]
    Menu,
    SelecionarTecido,
    SelecionarVinculo,
    Lancamento,
    Historico,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum VendaField {
    #[default]
    Tecido,
    Vinculo,
    Preco,
    Quantidade,
    Finalizar,
    Cancelar,
    Excluir,
}

impl VendaField {
    const ALL: [VendaField; 7] = [
        VendaField::Tecido,
        VendaField::Vinculo,
        VendaField::Preco,
        VendaField::Quantidade,
        VendaField::Finalizar,
        VendaField::Cancelar,
        VendaField::Excluir,
    ];

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0)
    }
}

#[derive(Clone)]
pub struct VendaItem {
    pub descricao: String,
    pub quantidade: f64,
    pub preco_unitario: f64,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum FinalizarVendaOption {
    #[default]
    Finalizar,
    FinalizarEImprimir,
}

impl FinalizarVendaOption {
    pub fn next(self) -> Self {
        match self {
            Self::Finalizar => Self::FinalizarEImprimir,
            Self::FinalizarEImprimir => Self::Finalizar,
        }
    }

    pub fn previous(self) -> Self {
        self.next()
    }
}

impl VendaItem {
    pub fn total(&self) -> f64 {
        self.quantidade * self.preco_unitario
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum DadosScreen {
    #[default]
    Menu,
    Tecidos,
    CadastrarTecido,
    Cores,
    CadastrarCor,
    Estampas,
    CadastrarEstampa,
    VinculosMenu,
    VinculosSelecionarTecidoCriar,
    VinculosSelecionarTecidoVer,
    VinculosSelecionarCores,
    VinculosLista,
    VinculoDetalhe,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum VinculoImageSlot {
    #[default]
    Original,
    Brand,
    Modelo,
    Alternativa,
}

impl VinculoImageSlot {
    pub const ALL: [VinculoImageSlot; 4] = [
        VinculoImageSlot::Original,
        VinculoImageSlot::Brand,
        VinculoImageSlot::Modelo,
        VinculoImageSlot::Alternativa,
    ];

    pub fn title(self) -> &'static str {
        match self {
            VinculoImageSlot::Original => "Imagem Original",
            VinculoImageSlot::Brand => "Imagem Brand",
            VinculoImageSlot::Modelo => "Imagem Modelo",
            VinculoImageSlot::Alternativa => "Imagem Alternativa",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            VinculoImageSlot::Original => "original",
            VinculoImageSlot::Brand => "brand",
            VinculoImageSlot::Modelo => "modelo",
            VinculoImageSlot::Alternativa => "alternativa",
        }
    }

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|slot| *slot == self).unwrap_or(0)
    }
}

#[derive(Default)]
pub struct CorForm {
    pub selected_field: CorField,
    pub hex: String,
    pub nome: String,
}

#[derive(Default)]
pub struct EstampaForm {
    pub selected_field: EstampaField,
    pub nome: String,
}

impl EstampaForm {
    pub fn push(&mut self, character: char) {
        if self.selected_field == EstampaField::Nome && !character.is_control() {
            self.nome.push(character);
        }
    }

    pub fn backspace(&mut self) {
        if self.selected_field == EstampaField::Nome {
            self.nome.pop();
        }
    }

    pub fn next_field(&mut self) {
        self.selected_field = self.selected_field.next();
    }

    pub fn previous_field(&mut self) {
        self.selected_field = self.selected_field.previous();
    }

    pub fn is_valid(&self) -> bool {
        !self.nome.trim().is_empty()
    }

    pub fn sku(&self, estampas: &[EstampaRecord], editing_id: Option<i64>) -> String {
        sku::build_estampa_sku(&self.nome, estampas, editing_id)
    }

    pub fn from_record(estampa: &EstampaRecord) -> Self {
        Self {
            selected_field: EstampaField::Nome,
            nome: estampa.nome.clone(),
        }
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum EstampaField {
    #[default]
    Nome,
    Confirmar,
    Voltar,
    Excluir,
}

impl EstampaField {
    const ALL: [EstampaField; 4] = [
        EstampaField::Nome,
        EstampaField::Confirmar,
        EstampaField::Voltar,
        EstampaField::Excluir,
    ];

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0)
    }
}

impl CorForm {
    pub fn push(&mut self, character: char) {
        if !self.selected_field.is_action() && !character.is_control() {
            self.current_value_mut().push(character);
        }
    }

    pub fn backspace(&mut self) {
        if !self.selected_field.is_action() {
            self.current_value_mut().pop();
        }
    }

    pub fn next_field(&mut self) {
        self.selected_field = self.selected_field.next();
    }

    pub fn previous_field(&mut self) {
        self.selected_field = self.selected_field.previous();
    }

    pub fn is_valid(&self) -> bool {
        !self.nome.trim().is_empty() && parse_hex_color(&self.hex).is_some()
    }

    pub fn sku(&self, cores: &[CorRecord], editing_id: Option<i64>) -> String {
        sku::build_cor_sku(&self.nome, cores, editing_id)
    }

    pub fn from_record(cor: &CorRecord) -> Self {
        Self {
            selected_field: CorField::Hex,
            hex: cor.codigo_hex.clone().unwrap_or_else(|| String::from("#")),
            nome: cor.nome.clone(),
        }
    }

    fn current_value_mut(&mut self) -> &mut String {
        match self.selected_field {
            CorField::Hex => &mut self.hex,
            CorField::Nome => &mut self.nome,
            CorField::Confirmar | CorField::Voltar | CorField::Excluir => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum CorField {
    #[default]
    Hex,
    Nome,
    Confirmar,
    Voltar,
    Excluir,
}

impl CorField {
    const ALL: [CorField; 5] = [
        CorField::Hex,
        CorField::Nome,
        CorField::Confirmar,
        CorField::Voltar,
        CorField::Excluir,
    ];

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0)
    }

    fn is_action(self) -> bool {
        matches!(
            self,
            CorField::Confirmar | CorField::Voltar | CorField::Excluir
        )
    }
}

#[derive(Default)]
pub struct TecidoForm {
    pub selected_field: TecidoField,
    pub nome: String,
    pub composicao: String,
    pub largura: String,
    pub tipo: SelectValue,
    pub transparencia: SelectValue,
    pub elasticidade: SelectValue,
    pub acabamento: SelectValue,
    pub rendimento: String,
    pub gramatura_linear: String,
    pub gramatura_m2: String,
}

impl TecidoForm {
    pub fn push(&mut self, character: char) {
        if self.selected_field.is_editable() && !character.is_control() {
            self.current_value_mut().push(character);
        }
    }

    pub fn backspace(&mut self) {
        if self.selected_field.is_editable() {
            self.current_value_mut().pop();
        }
    }

    pub fn next_field(&mut self) {
        self.selected_field = self.selected_field.next();
    }

    pub fn previous_field(&mut self) {
        self.selected_field = self.selected_field.previous();
    }

    pub fn sku(&self, tecidos: &[TecidoRecord], editing_id: Option<i64>) -> String {
        let skus = sku::existing_skus(tecidos, editing_id);
        let sku_refs: Vec<&str> = skus.iter().map(String::as_str).collect();
        sku::build_sku(&self.nome, &sku_refs)
    }

    pub fn calculated_values(&self) -> CalculatedTecidoValues {
        CalculatedTecidoValues::from_form(self)
    }

    pub fn is_valid(&self) -> bool {
        !self.nome.trim().is_empty()
            && !self.composicao.trim().is_empty()
            && parse_largura_m(&self.largura).is_some()
    }

    pub fn next_select_option(&mut self) {
        if let Some((value, options)) = self.current_select_mut() {
            value.next(options);
        }
    }

    pub fn previous_select_option(&mut self) {
        if let Some((value, options)) = self.current_select_mut() {
            value.previous(options);
        }
    }

    pub fn from_record(tecido: &TecidoRecord) -> Self {
        Self {
            selected_field: TecidoField::Nome,
            nome: tecido.nome.clone(),
            composicao: tecido.composicao.clone(),
            largura: format!("{:.2}m", tecido.largura_m),
            tipo: SelectValue::from_value(&tecido.tipo, TIPO_OPTIONS),
            transparencia: SelectValue::from_value(&tecido.transparencia, NIVEL_OPTIONS),
            elasticidade: SelectValue::from_value(&tecido.elasticidade, NIVEL_OPTIONS),
            acabamento: SelectValue::from_value(&tecido.acabamento, ACABAMENTO_OPTIONS),
            rendimento: tecido
                .rendimento_m_kg
                .map(|value| format!("{value:.2}"))
                .unwrap_or_default(),
            gramatura_linear: tecido
                .gramatura_linear_g_m
                .map(|value| value.to_string())
                .unwrap_or_default(),
            gramatura_m2: tecido
                .gramatura_g_m2
                .map(|value| value.to_string())
                .unwrap_or_default(),
        }
    }

    fn current_value_mut(&mut self) -> &mut String {
        match self.selected_field {
            TecidoField::Nome => &mut self.nome,
            TecidoField::Composicao => &mut self.composicao,
            TecidoField::Largura => &mut self.largura,
            TecidoField::Tipo
            | TecidoField::Transparencia
            | TecidoField::Elasticidade
            | TecidoField::Acabamento => unreachable!("selects nao possuem texto livre"),
            TecidoField::Rendimento => &mut self.rendimento,
            TecidoField::GramaturaLinear => &mut self.gramatura_linear,
            TecidoField::GramaturaM2 => &mut self.gramatura_m2,
            TecidoField::Salvar | TecidoField::Voltar | TecidoField::Excluir => {
                unreachable!("acoes nao possuem valor editavel")
            }
        }
    }

    fn current_select_mut(&mut self) -> Option<(&mut SelectValue, &'static [&'static str])> {
        match self.selected_field {
            TecidoField::Tipo => Some((&mut self.tipo, TIPO_OPTIONS)),
            TecidoField::Transparencia => Some((&mut self.transparencia, NIVEL_OPTIONS)),
            TecidoField::Elasticidade => Some((&mut self.elasticidade, NIVEL_OPTIONS)),
            TecidoField::Acabamento => Some((&mut self.acabamento, ACABAMENTO_OPTIONS)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum TecidoField {
    #[default]
    Nome,
    Composicao,
    Largura,
    Tipo,
    Transparencia,
    Elasticidade,
    Acabamento,
    Rendimento,
    GramaturaLinear,
    GramaturaM2,
    Salvar,
    Voltar,
    Excluir,
}

impl TecidoField {
    const ALL: [TecidoField; 13] = [
        TecidoField::Nome,
        TecidoField::Composicao,
        TecidoField::Largura,
        TecidoField::Tipo,
        TecidoField::Transparencia,
        TecidoField::Elasticidade,
        TecidoField::Acabamento,
        TecidoField::Rendimento,
        TecidoField::GramaturaLinear,
        TecidoField::GramaturaM2,
        TecidoField::Salvar,
        TecidoField::Voltar,
        TecidoField::Excluir,
    ];

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0)
    }

    pub fn is_select(self) -> bool {
        matches!(
            self,
            TecidoField::Tipo
                | TecidoField::Transparencia
                | TecidoField::Elasticidade
                | TecidoField::Acabamento
        )
    }

    fn is_editable(self) -> bool {
        !matches!(
            self,
            TecidoField::Tipo
                | TecidoField::Transparencia
                | TecidoField::Elasticidade
                | TecidoField::Acabamento
                | TecidoField::Salvar
                | TecidoField::Voltar
                | TecidoField::Excluir
        )
    }
}

pub const TIPO_OPTIONS: &[&str] = &["Selecione", "Nenhuma", "Liso", "Estampado"];
pub const NIVEL_OPTIONS: &[&str] = &["Selecione", "Nenhuma", "Baixa", "Media", "Alta"];
pub const ACABAMENTO_OPTIONS: &[&str] =
    &["Selecione", "Nenhuma", "Fosco", "Semi-brilho", "Brilhante"];

#[derive(Clone, Copy, Default)]
pub struct SelectValue {
    index: usize,
}

impl SelectValue {
    pub fn value(self, options: &'static [&'static str]) -> &'static str {
        options.get(self.index).copied().unwrap_or("Selecione")
    }

    pub fn from_value(value: &str, options: &'static [&'static str]) -> Self {
        let index = options
            .iter()
            .position(|option| option.eq_ignore_ascii_case(value))
            .unwrap_or(0);

        Self { index }
    }

    fn next(&mut self, options: &'static [&'static str]) {
        self.index = (self.index + 1) % options.len();
    }

    fn previous(&mut self, options: &'static [&'static str]) {
        self.index = (self.index + options.len() - 1) % options.len();
    }
}

#[derive(Default)]
pub struct CalculatedTecidoValues {
    pub rendimento: Option<f64>,
    pub gramatura_linear: Option<f64>,
    pub gramatura_m2: Option<f64>,
}

impl CalculatedTecidoValues {
    fn from_form(form: &TecidoForm) -> Self {
        let largura = parse_largura_m(&form.largura);
        let rendimento = parse_number(&form.rendimento);
        let gramatura_linear = parse_number(&form.gramatura_linear);
        let gramatura_m2 = parse_number(&form.gramatura_m2);

        let Some(largura_m) = largura.filter(|value| *value > 0.0) else {
            return Self {
                rendimento,
                gramatura_linear,
                gramatura_m2,
            };
        };

        if let Some(gl) = gramatura_linear.filter(|value| *value > 0.0) {
            return Self {
                rendimento: Some(1000.0 / gl),
                gramatura_linear: Some(gl),
                gramatura_m2: Some(gl / largura_m),
            };
        }

        if let Some(gm2) = gramatura_m2.filter(|value| *value > 0.0) {
            let gl = gm2 * largura_m;
            return Self {
                rendimento: Some(1000.0 / gl),
                gramatura_linear: Some(gl),
                gramatura_m2: Some(gm2),
            };
        }

        if let Some(rend) = rendimento.filter(|value| *value > 0.0) {
            let gl = 1000.0 / rend;
            return Self {
                rendimento: Some(rend),
                gramatura_linear: Some(gl),
                gramatura_m2: Some(gl / largura_m),
            };
        }

        Self::default()
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum DadosOption {
    #[default]
    Tecido,
    Cores,
    Estampas,
    Vinculos,
}

impl DadosOption {
    pub const ALL: [DadosOption; 4] = [
        DadosOption::Tecido,
        DadosOption::Cores,
        DadosOption::Estampas,
        DadosOption::Vinculos,
    ];

    pub fn title(self) -> &'static str {
        match self {
            DadosOption::Tecido => "Tecido",
            DadosOption::Cores => "Cores",
            DadosOption::Estampas => "Estampas",
            DadosOption::Vinculos => "Vinculos",
        }
    }

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|option| *option == self)
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum Section {
    #[default]
    Dashboard,
    Vendas,
    Pedidos,
    Dados,
    Estoque,
    Shopee,
    Documentos,
    Configuracoes,
}

impl Section {
    pub const ALL: [Section; 8] = [
        Section::Dashboard,
        Section::Vendas,
        Section::Pedidos,
        Section::Dados,
        Section::Estoque,
        Section::Shopee,
        Section::Documentos,
        Section::Configuracoes,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::Dashboard => "Dashboard",
            Section::Vendas => "Vendas",
            Section::Pedidos => "Pedidos",
            Section::Dados => "Dados",
            Section::Estoque => "Estoque",
            Section::Shopee => "Shopee",
            Section::Documentos => "Documentos",
            Section::Configuracoes => "Configuracoes",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|section| *section == self)
            .unwrap_or(0)
    }

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

pub fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() != 6 || !clean.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }

    let red = u8::from_str_radix(&clean[0..2], 16).ok()?;
    let green = u8::from_str_radix(&clean[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&clean[4..6], 16).ok()?;

    Some((red, green, blue))
}

pub fn is_complete_hex_color(hex: &str) -> bool {
    parse_hex_color(hex).is_some()
}

#[derive(Clone, Debug)]
pub struct ColorDistance {
    pub nome: String,
    pub sku: Option<String>,
    pub hex: String,
    pub delta_e: f64,
}

pub fn nearby_colors(
    hex: &str,
    cores: &[CorRecord],
    editing_id: Option<i64>,
    threshold: f64,
) -> Vec<ColorDistance> {
    let Some(rgb) = parse_hex_color(hex) else {
        return Vec::new();
    };
    let lab = rgb_to_lab(rgb);
    let mut matches = cores
        .iter()
        .filter(|cor| Some(cor.id) != editing_id)
        .filter_map(|cor| {
            let existing_hex = cor.codigo_hex.as_deref()?;
            let existing_lab = rgb_to_lab(parse_hex_color(existing_hex)?);
            let delta_e = ciede2000(lab, existing_lab);
            (delta_e < threshold).then(|| ColorDistance {
                nome: cor.nome.clone(),
                sku: cor.sku.clone(),
                hex: existing_hex.to_string(),
                delta_e,
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.delta_e.total_cmp(&right.delta_e));
    matches
}

pub fn closest_color_conflicts(cores: &[CorRecord], threshold: f64) -> Vec<Option<ColorDistance>> {
    let labs = cores
        .iter()
        .map(|cor| {
            cor.codigo_hex
                .as_deref()
                .and_then(parse_hex_color)
                .map(rgb_to_lab)
        })
        .collect::<Vec<_>>();
    let mut conflicts = vec![None; cores.len()];

    for left_index in 0..cores.len() {
        let Some(left_lab) = labs[left_index] else {
            continue;
        };
        for right_index in (left_index + 1)..cores.len() {
            let Some(right_lab) = labs[right_index] else {
                continue;
            };
            let delta_e = ciede2000(left_lab, right_lab);
            if delta_e < threshold {
                update_closest_conflict(&mut conflicts[left_index], &cores[right_index], delta_e);
                update_closest_conflict(&mut conflicts[right_index], &cores[left_index], delta_e);
            }
        }
    }

    conflicts
}

fn update_closest_conflict(current: &mut Option<ColorDistance>, color: &CorRecord, delta_e: f64) {
    if current
        .as_ref()
        .is_some_and(|existing| existing.delta_e <= delta_e)
    {
        return;
    }
    if let Some(hex) = &color.codigo_hex {
        *current = Some(ColorDistance {
            nome: color.nome.clone(),
            sku: color.sku.clone(),
            hex: hex.clone(),
            delta_e,
        });
    }
}

fn rgb_to_lab((red, green, blue): (u8, u8, u8)) -> (f64, f64, f64) {
    fn linearize(value: u8) -> f64 {
        let value = f64::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    let red = linearize(red);
    let green = linearize(green);
    let blue = linearize(blue);
    let x = (red * 0.4124564 + green * 0.3575761 + blue * 0.1804375) / 0.95047;
    let y = red * 0.2126729 + green * 0.7151522 + blue * 0.0721750;
    let z = (red * 0.0193339 + green * 0.1191920 + blue * 0.9503041) / 1.08883;

    fn lab_f(value: f64) -> f64 {
        if value > 216.0 / 24389.0 {
            value.cbrt()
        } else {
            (841.0 / 108.0) * value + 4.0 / 29.0
        }
    }

    let fx = lab_f(x);
    let fy = lab_f(y);
    let fz = lab_f(z);
    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

pub fn ciede2000(lab1: (f64, f64, f64), lab2: (f64, f64, f64)) -> f64 {
    let (l1, a1, b1) = lab1;
    let (l2, a2, b2) = lab2;
    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar = (c1 + c2) / 2.0;
    let c_bar7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + 25_f64.powi(7))).sqrt());
    let a1_prime = (1.0 + g) * a1;
    let a2_prime = (1.0 + g) * a2;
    let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
    let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();
    let h1_prime = hue_degrees(b1, a1_prime);
    let h2_prime = hue_degrees(b2, a2_prime);
    let delta_l_prime = l2 - l1;
    let delta_c_prime = c2_prime - c1_prime;
    let delta_h_prime = if c1_prime * c2_prime == 0.0 {
        0.0
    } else if (h2_prime - h1_prime).abs() <= 180.0 {
        h2_prime - h1_prime
    } else if h2_prime <= h1_prime {
        h2_prime - h1_prime + 360.0
    } else {
        h2_prime - h1_prime - 360.0
    };
    let delta_h_prime =
        2.0 * (c1_prime * c2_prime).sqrt() * degrees_to_radians(delta_h_prime / 2.0).sin();
    let l_bar_prime = (l1 + l2) / 2.0;
    let c_bar_prime = (c1_prime + c2_prime) / 2.0;
    let h_bar_prime = if c1_prime * c2_prime == 0.0 {
        h1_prime + h2_prime
    } else if (h1_prime - h2_prime).abs() <= 180.0 {
        (h1_prime + h2_prime) / 2.0
    } else if h1_prime + h2_prime < 360.0 {
        (h1_prime + h2_prime + 360.0) / 2.0
    } else {
        (h1_prime + h2_prime - 360.0) / 2.0
    };
    let t = 1.0 - 0.17 * degrees_to_radians(h_bar_prime - 30.0).cos()
        + 0.24 * degrees_to_radians(2.0 * h_bar_prime).cos()
        + 0.32 * degrees_to_radians(3.0 * h_bar_prime + 6.0).cos()
        - 0.20 * degrees_to_radians(4.0 * h_bar_prime - 63.0).cos();
    let delta_theta = 30.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp();
    let c_bar_prime7 = c_bar_prime.powi(7);
    let r_c = 2.0 * (c_bar_prime7 / (c_bar_prime7 + 25_f64.powi(7))).sqrt();
    let s_l =
        1.0 + (0.015 * (l_bar_prime - 50.0).powi(2)) / (20.0 + (l_bar_prime - 50.0).powi(2)).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_prime;
    let s_h = 1.0 + 0.015 * c_bar_prime * t;
    let r_t = -degrees_to_radians(2.0 * delta_theta).sin() * r_c;
    let l_term = delta_l_prime / s_l;
    let c_term = delta_c_prime / s_c;
    let h_term = delta_h_prime / s_h;
    (l_term * l_term + c_term * c_term + h_term * h_term + r_t * c_term * h_term).sqrt()
}

fn hue_degrees(b: f64, a: f64) -> f64 {
    let hue = b.atan2(a).to_degrees();
    if hue >= 0.0 { hue } else { hue + 360.0 }
}

fn degrees_to_radians(degrees: f64) -> f64 {
    degrees.to_radians()
}

pub fn round_to_nearest_ten(value: f64) -> i64 {
    (value / 10.0).round() as i64 * 10
}

pub fn parse_number(value: &str) -> Option<f64> {
    let mut normalized = String::new();
    let mut found_number = false;

    for character in value.trim().chars() {
        if character.is_ascii_digit() || character == ',' || character == '.' {
            normalized.push(if character == ',' { '.' } else { character });
            found_number = true;
        } else if found_number {
            break;
        }
    }

    normalized.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ciede2000_matches_reference_pair() {
        let delta = ciede2000((50.0, 2.6772, -79.7751), (50.0, 0.0, -82.7485));
        assert!((delta - 2.0425).abs() < 0.0001);
    }

    #[test]
    fn nearby_colors_respects_threshold_and_editing_id() {
        let cores = vec![
            CorRecord {
                id: 1,
                nome: String::from("Preto A"),
                sku: Some(String::from("PRA-01")),
                codigo_hex: Some(String::from("#101010")),
            },
            CorRecord {
                id: 2,
                nome: String::from("Branco"),
                sku: Some(String::from("BRAN-01")),
                codigo_hex: Some(String::from("#FFFFFF")),
            },
        ];

        let matches = nearby_colors("#111111", &cores, None, 3.0);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].nome, "Preto A");

        let matches = nearby_colors("#111111", &cores, Some(1), 3.0);
        assert!(matches.is_empty());
    }
}

pub fn parse_largura_m(value: &str) -> Option<f64> {
    let number = parse_number(value)?;
    let normalized = value.trim().to_lowercase();

    if normalized.contains("cm") {
        Some(number / 100.0)
    } else if normalized.contains('m') || number <= 10.0 {
        Some(number)
    } else {
        Some(number / 100.0)
    }
}
