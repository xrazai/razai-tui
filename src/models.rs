mod sku;
pub use sku::{build_estampa_vinculo_sku, build_vinculo_sku};

use crate::db::{CorRecord, EstampaRecord, TecidoRecord};

mod core;
pub use core::{ChatMessage, ChatState, Focus};

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
    Configuracoes,
}

impl Section {
    pub const ALL: [Section; 6] = [
        Section::Dashboard,
        Section::Vendas,
        Section::Pedidos,
        Section::Dados,
        Section::Estoque,
        Section::Configuracoes,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::Dashboard => "Dashboard",
            Section::Vendas => "Vendas",
            Section::Pedidos => "Pedidos",
            Section::Dados => "Dados",
            Section::Estoque => "Estoque",
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
