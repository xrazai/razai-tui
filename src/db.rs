use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use crate::models::{
    ACABAMENTO_OPTIONS, NIVEL_OPTIONS, TIPO_OPTIONS, TecidoForm, parse_largura_m,
    round_to_nearest_ten,
};

pub async fn connect() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| String::from("postgres://razai:razai_dev@localhost:5432/razai_tui"));

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}

#[derive(Clone)]
pub struct TecidoRecord {
    pub id: i64,
    pub nome: String,
    pub sku: String,
    pub composicao: String,
    pub largura_m: f64,
    pub rendimento_m_kg: Option<f64>,
    pub gramatura_linear_g_m: Option<i32>,
    pub gramatura_g_m2: Option<i32>,
    pub tipo: String,
    pub transparencia: String,
    pub elasticidade: String,
    pub acabamento: String,
}

#[derive(Clone)]
pub struct CorRecord {
    pub id: i64,
    pub nome: String,
    pub sku: Option<String>,
    pub codigo_hex: Option<String>,
}

#[derive(Clone)]
pub struct VinculoRecord {
    pub cor_id: i64,
    pub tecido_nome: String,
    pub cor_nome: String,
    pub cor_hex: Option<String>,
    pub sku: Option<String>,
}

pub async fn list_tecidos(pool: &PgPool) -> Result<Vec<TecidoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            id,
            nome,
            sku,
            composicao,
            largura_m::float8 AS largura_m,
            rendimento_m_kg::float8 AS rendimento_m_kg,
            gramatura_linear_g_m,
            gramatura_g_m2,
            tipo,
            transparencia,
            elasticidade,
            acabamento
        FROM tecidos
        ORDER BY nome, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| TecidoRecord {
            id: row.get("id"),
            nome: row.get("nome"),
            sku: row.get("sku"),
            composicao: row.get("composicao"),
            largura_m: row.get("largura_m"),
            rendimento_m_kg: row.get("rendimento_m_kg"),
            gramatura_linear_g_m: row.get("gramatura_linear_g_m"),
            gramatura_g_m2: row.get("gramatura_g_m2"),
            tipo: row.get("tipo"),
            transparencia: row.get("transparencia"),
            elasticidade: row.get("elasticidade"),
            acabamento: row.get("acabamento"),
        })
        .collect())
}

pub async fn insert_tecido(pool: &PgPool, form: &TecidoForm, sku: &str) -> Result<(), sqlx::Error> {
    let calculated = form.calculated_values();
    let largura_m = parse_largura_m(&form.largura).unwrap_or_default();
    let rendimento = calculated.rendimento;
    let gramatura_linear = rounded_gramatura(calculated.gramatura_linear);
    let gramatura_m2 = rounded_gramatura(calculated.gramatura_m2);

    sqlx::query(
        r#"
        INSERT INTO tecidos (
            nome,
            sku,
            composicao,
            largura_m,
            rendimento_m_kg,
            gramatura_linear_g_m,
            gramatura_g_m2,
            tipo,
            transparencia,
            elasticidade,
            acabamento
        )
        VALUES ($1, $2, $3, $4::numeric, $5::numeric, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(form.nome.trim())
    .bind(sku)
    .bind(form.composicao.trim())
    .bind(largura_m)
    .bind(rendimento)
    .bind(gramatura_linear)
    .bind(gramatura_m2)
    .bind(form.tipo.value(TIPO_OPTIONS))
    .bind(form.transparencia.value(NIVEL_OPTIONS))
    .bind(form.elasticidade.value(NIVEL_OPTIONS))
    .bind(form.acabamento.value(ACABAMENTO_OPTIONS))
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_tecido(
    pool: &PgPool,
    id: i64,
    form: &TecidoForm,
    sku: &str,
) -> Result<(), sqlx::Error> {
    let calculated = form.calculated_values();
    let largura_m = parse_largura_m(&form.largura).unwrap_or_default();
    let rendimento = calculated.rendimento;
    let gramatura_linear = rounded_gramatura(calculated.gramatura_linear);
    let gramatura_m2 = rounded_gramatura(calculated.gramatura_m2);

    sqlx::query(
        r#"
        UPDATE tecidos
        SET
            nome = $1,
            sku = $2,
            composicao = $3,
            largura_m = $4::numeric,
            rendimento_m_kg = $5::numeric,
            gramatura_linear_g_m = $6,
            gramatura_g_m2 = $7,
            tipo = $8,
            transparencia = $9,
            elasticidade = $10,
            acabamento = $11
        WHERE id = $12
        "#,
    )
    .bind(form.nome.trim())
    .bind(sku)
    .bind(form.composicao.trim())
    .bind(largura_m)
    .bind(rendimento)
    .bind(gramatura_linear)
    .bind(gramatura_m2)
    .bind(form.tipo.value(TIPO_OPTIONS))
    .bind(form.transparencia.value(NIVEL_OPTIONS))
    .bind(form.elasticidade.value(NIVEL_OPTIONS))
    .bind(form.acabamento.value(ACABAMENTO_OPTIONS))
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_tecido(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM tecidos WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn list_cores(pool: &PgPool) -> Result<Vec<CorRecord>, sqlx::Error> {
    let rows = sqlx::query("SELECT id, nome, sku, codigo_hex FROM cores ORDER BY nome, id")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| CorRecord {
            id: row.get("id"),
            nome: row.get("nome"),
            sku: row.get("sku"),
            codigo_hex: row.get("codigo_hex"),
        })
        .collect())
}

pub async fn insert_cor(
    pool: &PgPool,
    nome: &str,
    sku: &str,
    codigo_hex: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO cores (nome, sku, codigo_hex) VALUES ($1, $2, $3)")
        .bind(nome.trim())
        .bind(sku)
        .bind(normalize_hex(codigo_hex))
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_cor(
    pool: &PgPool,
    id: i64,
    nome: &str,
    sku: &str,
    codigo_hex: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE cores SET nome = $1, sku = $2, codigo_hex = $3 WHERE id = $4")
        .bind(nome.trim())
        .bind(sku)
        .bind(normalize_hex(codigo_hex))
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_cor(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM cores WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn list_vinculos_by_tecido(
    pool: &PgPool,
    tecido_id: i64,
) -> Result<Vec<VinculoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            tc.cor_id,
            t.nome AS tecido_nome,
            c.nome AS cor_nome,
            c.codigo_hex AS cor_hex,
            tc.sku
        FROM tecido_cores tc
        JOIN tecidos t ON t.id = tc.tecido_id
        JOIN cores c ON c.id = tc.cor_id
        WHERE tc.tecido_id = $1
        ORDER BY c.nome, c.id
        "#,
    )
    .bind(tecido_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| VinculoRecord {
            cor_id: row.get("cor_id"),
            tecido_nome: row.get("tecido_nome"),
            cor_nome: row.get("cor_nome"),
            cor_hex: row.get("cor_hex"),
            sku: row.get("sku"),
        })
        .collect())
}

pub async fn list_vinculos_by_tecido_and_tipo(
    pool: &PgPool,
    tecido_id: i64,
    tipo: &str,
) -> Result<Vec<VinculoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            tc.cor_id,
            t.nome AS tecido_nome,
            c.nome AS cor_nome,
            c.codigo_hex AS cor_hex,
            tc.sku
        FROM tecido_cores tc
        JOIN tecidos t ON t.id = tc.tecido_id
        JOIN cores c ON c.id = tc.cor_id
        WHERE tc.tecido_id = $1
          AND t.tipo = $2
        ORDER BY c.nome, c.id
        "#,
    )
    .bind(tecido_id)
    .bind(tipo)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| VinculoRecord {
            cor_id: row.get("cor_id"),
            tecido_nome: row.get("tecido_nome"),
            cor_nome: row.get("cor_nome"),
            cor_hex: row.get("cor_hex"),
            sku: row.get("sku"),
        })
        .collect())
}

pub async fn replace_vinculos(
    pool: &PgPool,
    tecido_id: i64,
    vinculos: &[(i64, String)],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;

    sqlx::query("DELETE FROM tecido_cores WHERE tecido_id = $1")
        .bind(tecido_id)
        .execute(&mut *transaction)
        .await?;

    for (cor_id, sku) in vinculos {
        sqlx::query("INSERT INTO tecido_cores (tecido_id, cor_id, sku) VALUES ($1, $2, $3)")
            .bind(tecido_id)
            .bind(cor_id)
            .bind(sku)
            .execute(&mut *transaction)
            .await?;
    }

    transaction.commit().await?;

    Ok(())
}

fn normalize_hex(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('#') {
        trimmed.to_string()
    } else {
        format!("#{trimmed}")
    }
}

fn rounded_gramatura(value: Option<f64>) -> Option<i32> {
    value.map(|number| round_to_nearest_ten(number) as i32)
}
