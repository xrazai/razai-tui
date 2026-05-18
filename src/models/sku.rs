use crate::db::{CorRecord, EstampaRecord, TecidoRecord};

pub fn build_vinculo_sku(tecido: &TecidoRecord, cor: &CorRecord) -> String {
    let cor_sku = cor.sku.as_deref().unwrap_or("____-__");
    format!("{}-{}", tecido.sku, cor_sku)
}

pub fn build_estampa_vinculo_sku(tecido: &TecidoRecord, estampa: &EstampaRecord) -> String {
    format!(
        "{}-{}",
        tecido.sku,
        estampa.sku.as_deref().unwrap_or("ESTA-01")
    )
}

pub(super) fn existing_skus(tecidos: &[TecidoRecord], editing_id: Option<i64>) -> Vec<String> {
    tecidos
        .iter()
        .filter(|tecido| Some(tecido.id) != editing_id)
        .map(|tecido| {
            if tecido.sku.trim().is_empty() {
                build_sku(&tecido.nome, &[])
            } else {
                tecido.sku.clone()
            }
        })
        .collect()
}

pub(super) fn build_cor_sku(name: &str, cores: &[CorRecord], editing_id: Option<i64>) -> String {
    let prefix = build_cor_sku_prefix(name);
    let next_sequence = cores
        .iter()
        .filter(|cor| Some(cor.id) != editing_id)
        .filter_map(|cor| cor.sku.as_deref())
        .filter(|sku| sku.starts_with(&prefix))
        .filter_map(|sku| sku.rsplit_once('-')?.1.parse::<u16>().ok())
        .max()
        .unwrap_or(0)
        + 1;

    format!("{prefix}-{next_sequence:02}")
}

pub(super) fn build_estampa_sku(
    name: &str,
    estampas: &[EstampaRecord],
    editing_id: Option<i64>,
) -> String {
    let base = build_cor_sku_prefix(name);
    let next_sequence = estampas
        .iter()
        .filter(|estampa| Some(estampa.id) != editing_id)
        .filter_map(|estampa| estampa.sku.as_deref())
        .filter(|sku| sku.starts_with(&base))
        .filter_map(|sku| sku.rsplit_once('-')?.1.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;

    format!("{base}-{next_sequence:02}")
}

pub(super) fn build_cor_sku_prefix(name: &str) -> String {
    let words: Vec<String> = name
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .collect::<String>()
                .to_uppercase()
        })
        .filter(|word| !word.is_empty())
        .collect();

    let family = words.first().map(String::as_str).unwrap_or("");
    let color = words.last().map(String::as_str).unwrap_or(family);

    pad_sku(format!(
        "{}{}",
        first_chars(family, 2),
        first_chars(color, 2)
    ))
}

pub(super) fn build_sku(name: &str, existing_skus: &[&str]) -> String {
    let words: Vec<String> = name
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .collect::<String>()
                .to_uppercase()
        })
        .filter(|word| !word.is_empty())
        .collect();

    if words.is_empty() {
        return String::from("____");
    }

    let mut sku = if words.len() == 1 {
        first_chars(&words[0], 4)
    } else {
        format!(
            "{}{}",
            first_chars(words.first().unwrap(), 2),
            first_chars(words.last().unwrap(), 2)
        )
    };

    sku = pad_sku(sku);

    if !existing_skus.iter().any(|existing| *existing == sku) {
        return sku;
    }

    if let Some(word) = words.first() {
        for character in word.chars().skip(3) {
            let mut candidate = sku.clone();
            candidate.replace_range(3..4, &character.to_string());
            if !existing_skus.iter().any(|existing| *existing == candidate) {
                return candidate;
            }
        }
    }

    sku
}

pub(super) fn first_chars(value: &str, count: usize) -> String {
    value.chars().take(count).collect()
}

pub(super) fn pad_sku(mut sku: String) -> String {
    while sku.len() < 4 {
        sku.push('X');
    }
    sku.chars().take(4).collect()
}
