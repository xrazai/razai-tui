use sqlx::{PgPool, Row, postgres::PgPoolOptions};

mod orders;
mod sales;
mod stock;
pub use orders::*;
pub use sales::*;
pub use stock::*;

use crate::models::{
    ACABAMENTO_OPTIONS, ListaPrecoTipo, NIVEL_OPTIONS, TIPO_OPTIONS, TecidoForm, parse_largura_m,
    parse_number, round_to_nearest_ten,
};

pub async fn connect() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| String::from("postgres://razai:razai_dev@localhost:5432/razai_tui"));

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}

pub async fn ensure_configuracoes_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS configuracoes (
            chave TEXT PRIMARY KEY,
            valor TEXT NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO configuracoes (chave, valor, updated_at)
        VALUES ('color_delta_e_threshold', '3', NOW())
        ON CONFLICT (chave) DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn ensure_fornecedores_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS fornecedores (
            id BIGSERIAL PRIMARY KEY,
            nome TEXT NOT NULL,
            empresa TEXT NOT NULL DEFAULT '',
            telefone TEXT NOT NULL DEFAULT '',
            endereco TEXT NOT NULL DEFAULT '',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn ensure_tecido_custo_base_column(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("ALTER TABLE tecidos ADD COLUMN IF NOT EXISTS custo_base NUMERIC(12, 2)")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE tecidos ADD COLUMN IF NOT EXISTS preco_atacado NUMERIC(12, 2)")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE tecidos ADD COLUMN IF NOT EXISTS preco_varejo NUMERIC(12, 2)")
        .execute(pool)
        .await?;
    sqlx::query(
        "ALTER TABLE tecidos ADD COLUMN IF NOT EXISTS fornecedor_id BIGINT REFERENCES fornecedores(id) ON DELETE SET NULL",
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn ensure_estampas_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS estampas (
            id BIGSERIAL PRIMARY KEY,
            nome TEXT NOT NULL,
            sku TEXT UNIQUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tecido_estampas (
            id BIGSERIAL PRIMARY KEY,
            tecido_id BIGINT NOT NULL REFERENCES tecidos(id) ON DELETE CASCADE,
            estampa_id BIGINT NOT NULL REFERENCES estampas(id) ON DELETE RESTRICT,
            sku TEXT UNIQUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (tecido_id, estampa_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tecido_estampas_tecido_id ON tecido_estampas(tecido_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tecido_estampas_estampa_id ON tecido_estampas(estampa_id)",
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn ensure_vendas_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS vendas (
            id BIGSERIAL PRIMARY KEY,
            total NUMERIC(12, 2) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS venda_itens (
            id BIGSERIAL PRIMARY KEY,
            venda_id BIGINT NOT NULL REFERENCES vendas(id) ON DELETE CASCADE,
            descricao TEXT NOT NULL,
            quantidade NUMERIC(12, 3) NOT NULL,
            preco_unitario NUMERIC(12, 2) NOT NULL,
            subtotal NUMERIC(12, 2) NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_venda_itens_venda_id ON venda_itens(venda_id)")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE venda_itens ADD COLUMN IF NOT EXISTS estoque_tecido_id BIGINT")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE venda_itens ADD COLUMN IF NOT EXISTS estoque_item_id BIGINT")
        .execute(pool)
        .await?;
    sqlx::query(
        "ALTER TABLE venda_itens ADD COLUMN IF NOT EXISTS estoque_usa_estampas BOOLEAN NOT NULL DEFAULT FALSE",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_vendas_created_at ON vendas(created_at DESC)")
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn ensure_pedidos_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS pedidos (
            id BIGSERIAL PRIMARY KEY,
            total NUMERIC(12, 2) NOT NULL,
            status TEXT NOT NULL DEFAULT 'pendente',
            pdf_path TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS pedido_itens (
            id BIGSERIAL PRIMARY KEY,
            pedido_id BIGINT NOT NULL REFERENCES pedidos(id) ON DELETE CASCADE,
            descricao TEXT NOT NULL,
            quantidade NUMERIC(12, 3) NOT NULL,
            preco_unitario NUMERIC(12, 2) NOT NULL,
            subtotal NUMERIC(12, 2) NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pedido_itens_pedido_id ON pedido_itens(pedido_id)")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE pedido_itens ADD COLUMN IF NOT EXISTS estoque_tecido_id BIGINT")
        .execute(pool)
        .await?;
    sqlx::query("ALTER TABLE pedido_itens ADD COLUMN IF NOT EXISTS estoque_item_id BIGINT")
        .execute(pool)
        .await?;
    sqlx::query(
        "ALTER TABLE pedido_itens ADD COLUMN IF NOT EXISTS estoque_usa_estampas BOOLEAN NOT NULL DEFAULT FALSE",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pedidos_created_at ON pedidos(created_at DESC)")
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn ensure_vinculo_image_columns(pool: &PgPool) -> Result<(), sqlx::Error> {
    for table in ["tecido_cores", "tecido_estampas"] {
        for column in [
            "imagem_original",
            "imagem_brand",
            "imagem_modelo",
            "imagem_alternativa",
        ] {
            sqlx::query(&format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS {column} BYTEA"
            ))
            .execute(pool)
            .await?;
        }
        sqlx::query(&format!(
            "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS ativo BOOLEAN NOT NULL DEFAULT TRUE"
        ))
        .execute(pool)
        .await?;
        sqlx::query(&format!(
            "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS custo_override NUMERIC(12, 2)"
        ))
        .execute(pool)
        .await?;
        sqlx::query(&format!(
            "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS preco_atacado_override NUMERIC(12, 2)"
        ))
        .execute(pool)
        .await?;
        sqlx::query(&format!(
            "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS preco_varejo_override NUMERIC(12, 2)"
        ))
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_config(pool: &PgPool, chave: &str) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT valor FROM configuracoes WHERE chave = $1")
        .bind(chave)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|row| row.get("valor")))
}

pub async fn set_config(pool: &PgPool, chave: &str, valor: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO configuracoes (chave, valor, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (chave)
        DO UPDATE SET valor = EXCLUDED.valor, updated_at = NOW()
        "#,
    )
    .bind(chave)
    .bind(valor)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Clone)]
pub struct TecidoRecord {
    pub id: i64,
    pub nome: String,
    pub sku: String,
    pub composicao: String,
    pub largura_m: f64,
    pub custo_base: Option<f64>,
    pub preco_atacado: Option<f64>,
    pub preco_varejo: Option<f64>,
    pub custo_override_count: i64,
    pub preco_atacado_override_count: i64,
    pub preco_varejo_override_count: i64,
    pub custo_override_min: Option<f64>,
    pub custo_override_max: Option<f64>,
    pub preco_atacado_override_min: Option<f64>,
    pub preco_atacado_override_max: Option<f64>,
    pub preco_varejo_override_min: Option<f64>,
    pub preco_varejo_override_max: Option<f64>,
    pub rendimento_m_kg: Option<f64>,
    pub gramatura_linear_g_m: Option<i32>,
    pub gramatura_g_m2: Option<i32>,
    pub tipo: String,
    pub transparencia: String,
    pub elasticidade: String,
    pub acabamento: String,
    pub fornecedor_id: Option<i64>,
    pub fornecedor_nome: Option<String>,
}

#[derive(Clone)]
pub struct CorRecord {
    pub id: i64,
    pub nome: String,
    pub sku: Option<String>,
    pub codigo_hex: Option<String>,
}

#[derive(Clone)]
pub struct EstampaRecord {
    pub id: i64,
    pub nome: String,
    pub sku: Option<String>,
}

#[derive(Clone)]
pub struct FornecedorRecord {
    pub id: i64,
    pub nome: String,
    pub empresa: String,
    pub telefone: String,
    pub endereco: String,
}

#[derive(Clone)]
pub struct VinculoRecord {
    pub cor_id: i64,
    pub tecido_nome: String,
    pub cor_nome: String,
    pub cor_hex: Option<String>,
    pub sku: Option<String>,
    pub tecido_custo_base: Option<f64>,
    pub custo_override: Option<f64>,
    pub custo_efetivo: Option<f64>,
    pub tecido_preco_atacado: Option<f64>,
    pub tecido_preco_varejo: Option<f64>,
    pub preco_atacado_override: Option<f64>,
    pub preco_varejo_override: Option<f64>,
    pub preco_atacado_efetivo: Option<f64>,
    pub preco_varejo_efetivo: Option<f64>,
    pub has_imagem_original: bool,
    pub has_imagem_brand: bool,
    pub has_imagem_modelo: bool,
    pub has_imagem_alternativa: bool,
}

#[derive(Clone, Default)]
pub struct VinculoImages {
    pub imagem_original: Option<Vec<u8>>,
    pub imagem_brand: Option<Vec<u8>>,
    pub imagem_modelo: Option<Vec<u8>>,
    pub imagem_alternativa: Option<Vec<u8>>,
}

pub async fn list_tecidos(pool: &PgPool) -> Result<Vec<TecidoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            tecidos.id,
            tecidos.nome,
            tecidos.sku,
            tecidos.composicao,
            tecidos.largura_m::float8 AS largura_m,
            tecidos.custo_base::float8 AS custo_base,
            tecidos.preco_atacado::float8 AS preco_atacado,
            tecidos.preco_varejo::float8 AS preco_varejo,
            (
                COALESCE(tc_counts.custo_override_count, 0)
                + COALESCE(te_counts.custo_override_count, 0)
            ) AS custo_override_count,
            (
                COALESCE(tc_counts.preco_atacado_override_count, 0)
                + COALESCE(te_counts.preco_atacado_override_count, 0)
            ) AS preco_atacado_override_count,
            (
                COALESCE(tc_counts.preco_varejo_override_count, 0)
                + COALESCE(te_counts.preco_varejo_override_count, 0)
            ) AS preco_varejo_override_count,
            LEAST(tc_counts.custo_override_min, te_counts.custo_override_min)::float8 AS custo_override_min,
            GREATEST(tc_counts.custo_override_max, te_counts.custo_override_max)::float8 AS custo_override_max,
            LEAST(tc_counts.preco_atacado_override_min, te_counts.preco_atacado_override_min)::float8 AS preco_atacado_override_min,
            GREATEST(tc_counts.preco_atacado_override_max, te_counts.preco_atacado_override_max)::float8 AS preco_atacado_override_max,
            LEAST(tc_counts.preco_varejo_override_min, te_counts.preco_varejo_override_min)::float8 AS preco_varejo_override_min,
            GREATEST(tc_counts.preco_varejo_override_max, te_counts.preco_varejo_override_max)::float8 AS preco_varejo_override_max,
            tecidos.rendimento_m_kg::float8 AS rendimento_m_kg,
            tecidos.gramatura_linear_g_m,
            tecidos.gramatura_g_m2,
            tecidos.tipo,
            tecidos.transparencia,
            tecidos.elasticidade,
            tecidos.acabamento,
            tecidos.fornecedor_id,
            f.nome AS fornecedor_nome
        FROM tecidos
        LEFT JOIN fornecedores f ON f.id = tecidos.fornecedor_id
        LEFT JOIN (
            SELECT
                tecido_id,
                COUNT(*) FILTER (WHERE custo_override IS NOT NULL) AS custo_override_count,
                COUNT(*) FILTER (WHERE preco_atacado_override IS NOT NULL) AS preco_atacado_override_count,
                COUNT(*) FILTER (WHERE preco_varejo_override IS NOT NULL) AS preco_varejo_override_count,
                MIN(custo_override) AS custo_override_min,
                MAX(custo_override) AS custo_override_max,
                MIN(preco_atacado_override) AS preco_atacado_override_min,
                MAX(preco_atacado_override) AS preco_atacado_override_max,
                MIN(preco_varejo_override) AS preco_varejo_override_min,
                MAX(preco_varejo_override) AS preco_varejo_override_max
            FROM tecido_cores
            WHERE ativo = TRUE
            GROUP BY tecido_id
        ) tc_counts ON tc_counts.tecido_id = tecidos.id
        LEFT JOIN (
            SELECT
                tecido_id,
                COUNT(*) FILTER (WHERE custo_override IS NOT NULL) AS custo_override_count,
                COUNT(*) FILTER (WHERE preco_atacado_override IS NOT NULL) AS preco_atacado_override_count,
                COUNT(*) FILTER (WHERE preco_varejo_override IS NOT NULL) AS preco_varejo_override_count,
                MIN(custo_override) AS custo_override_min,
                MAX(custo_override) AS custo_override_max,
                MIN(preco_atacado_override) AS preco_atacado_override_min,
                MAX(preco_atacado_override) AS preco_atacado_override_max,
                MIN(preco_varejo_override) AS preco_varejo_override_min,
                MAX(preco_varejo_override) AS preco_varejo_override_max
            FROM tecido_estampas
            WHERE ativo = TRUE
            GROUP BY tecido_id
        ) te_counts ON te_counts.tecido_id = tecidos.id
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
            custo_base: row.get("custo_base"),
            preco_atacado: row.get("preco_atacado"),
            preco_varejo: row.get("preco_varejo"),
            custo_override_count: row.get("custo_override_count"),
            preco_atacado_override_count: row.get("preco_atacado_override_count"),
            preco_varejo_override_count: row.get("preco_varejo_override_count"),
            custo_override_min: row.get("custo_override_min"),
            custo_override_max: row.get("custo_override_max"),
            preco_atacado_override_min: row.get("preco_atacado_override_min"),
            preco_atacado_override_max: row.get("preco_atacado_override_max"),
            preco_varejo_override_min: row.get("preco_varejo_override_min"),
            preco_varejo_override_max: row.get("preco_varejo_override_max"),
            rendimento_m_kg: row.get("rendimento_m_kg"),
            gramatura_linear_g_m: row.get("gramatura_linear_g_m"),
            gramatura_g_m2: row.get("gramatura_g_m2"),
            tipo: row.get("tipo"),
            transparencia: row.get("transparencia"),
            elasticidade: row.get("elasticidade"),
            acabamento: row.get("acabamento"),
            fornecedor_id: row.get("fornecedor_id"),
            fornecedor_nome: row.get("fornecedor_nome"),
        })
        .collect())
}

pub async fn insert_tecido(
    pool: &PgPool,
    form: &TecidoForm,
    sku: &str,
    fornecedor_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    let calculated = form.calculated_values();
    let largura_m = parse_largura_m(&form.largura).unwrap_or_default();
    let custo_base = parse_number(&form.custo_base).filter(|value| *value >= 0.0);
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
            custo_base,
            rendimento_m_kg,
            gramatura_linear_g_m,
            gramatura_g_m2,
            tipo,
            transparencia,
            elasticidade,
            acabamento,
            fornecedor_id
        )
        VALUES ($1, $2, $3, $4::numeric, $5::numeric, $6::numeric, $7, $8, $9, $10, $11, $12, $13)
        "#,
    )
    .bind(form.nome.trim())
    .bind(sku)
    .bind(form.composicao.trim())
    .bind(largura_m)
    .bind(custo_base)
    .bind(rendimento)
    .bind(gramatura_linear)
    .bind(gramatura_m2)
    .bind(form.tipo.value(TIPO_OPTIONS))
    .bind(form.transparencia.value(NIVEL_OPTIONS))
    .bind(form.elasticidade.value(NIVEL_OPTIONS))
    .bind(form.acabamento.value(ACABAMENTO_OPTIONS))
    .bind(fornecedor_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_tecido(
    pool: &PgPool,
    id: i64,
    form: &TecidoForm,
    sku: &str,
    fornecedor_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    let calculated = form.calculated_values();
    let largura_m = parse_largura_m(&form.largura).unwrap_or_default();
    let custo_base = parse_number(&form.custo_base).filter(|value| *value >= 0.0);
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
            custo_base = $5::numeric,
            rendimento_m_kg = $6::numeric,
            gramatura_linear_g_m = $7,
            gramatura_g_m2 = $8,
            tipo = $9,
            transparencia = $10,
            elasticidade = $11,
            acabamento = $12,
            fornecedor_id = $13
        WHERE id = $14
        "#,
    )
    .bind(form.nome.trim())
    .bind(sku)
    .bind(form.composicao.trim())
    .bind(largura_m)
    .bind(custo_base)
    .bind(rendimento)
    .bind(gramatura_linear)
    .bind(gramatura_m2)
    .bind(form.tipo.value(TIPO_OPTIONS))
    .bind(form.transparencia.value(NIVEL_OPTIONS))
    .bind(form.elasticidade.value(NIVEL_OPTIONS))
    .bind(form.acabamento.value(ACABAMENTO_OPTIONS))
    .bind(fornecedor_id)
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

pub async fn list_fornecedores(pool: &PgPool) -> Result<Vec<FornecedorRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, nome, empresa, telefone, endereco
        FROM fornecedores
        ORDER BY nome, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| FornecedorRecord {
            id: row.get("id"),
            nome: row.get("nome"),
            empresa: row.get("empresa"),
            telefone: row.get("telefone"),
            endereco: row.get("endereco"),
        })
        .collect())
}

pub async fn insert_fornecedor(
    pool: &PgPool,
    nome: &str,
    empresa: &str,
    telefone: &str,
    endereco: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO fornecedores (nome, empresa, telefone, endereco) VALUES ($1, $2, $3, $4)",
    )
    .bind(nome.trim())
    .bind(empresa.trim())
    .bind(telefone.trim())
    .bind(endereco.trim())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_fornecedor(
    pool: &PgPool,
    id: i64,
    nome: &str,
    empresa: &str,
    telefone: &str,
    endereco: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE fornecedores SET nome = $1, empresa = $2, telefone = $3, endereco = $4 WHERE id = $5",
    )
    .bind(nome.trim())
    .bind(empresa.trim())
    .bind(telefone.trim())
    .bind(endereco.trim())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_fornecedor(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM fornecedores WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_estampas(pool: &PgPool) -> Result<Vec<EstampaRecord>, sqlx::Error> {
    let rows = sqlx::query("SELECT id, nome, sku FROM estampas ORDER BY nome, id")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| EstampaRecord {
            id: row.get("id"),
            nome: row.get("nome"),
            sku: row.get("sku"),
        })
        .collect())
}

pub async fn insert_estampa(pool: &PgPool, nome: &str, sku: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO estampas (nome, sku) VALUES ($1, $2)")
        .bind(nome.trim())
        .bind(sku)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_estampa(
    pool: &PgPool,
    id: i64,
    nome: &str,
    sku: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE estampas SET nome = $1, sku = $2 WHERE id = $3")
        .bind(nome.trim())
        .bind(sku)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_estampa(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM estampas WHERE id = $1")
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
            tc.sku,
            t.custo_base::float8 AS tecido_custo_base,
            tc.custo_override::float8 AS custo_override,
            COALESCE(tc.custo_override, t.custo_base)::float8 AS custo_efetivo,
            t.preco_atacado::float8 AS tecido_preco_atacado,
            t.preco_varejo::float8 AS tecido_preco_varejo,
            tc.preco_atacado_override::float8 AS preco_atacado_override,
            tc.preco_varejo_override::float8 AS preco_varejo_override,
            COALESCE(tc.preco_atacado_override, t.preco_atacado)::float8 AS preco_atacado_efetivo,
            COALESCE(tc.preco_varejo_override, t.preco_varejo)::float8 AS preco_varejo_efetivo,
            tc.imagem_original IS NOT NULL AS has_imagem_original,
            tc.imagem_brand IS NOT NULL AS has_imagem_brand,
            tc.imagem_modelo IS NOT NULL AS has_imagem_modelo,
            tc.imagem_alternativa IS NOT NULL AS has_imagem_alternativa
        FROM tecido_cores tc
        JOIN tecidos t ON t.id = tc.tecido_id
        JOIN cores c ON c.id = tc.cor_id
        WHERE tc.tecido_id = $1 AND tc.ativo = TRUE
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
            tecido_custo_base: row.get("tecido_custo_base"),
            custo_override: row.get("custo_override"),
            custo_efetivo: row.get("custo_efetivo"),
            tecido_preco_atacado: row.get("tecido_preco_atacado"),
            tecido_preco_varejo: row.get("tecido_preco_varejo"),
            preco_atacado_override: row.get("preco_atacado_override"),
            preco_varejo_override: row.get("preco_varejo_override"),
            preco_atacado_efetivo: row.get("preco_atacado_efetivo"),
            preco_varejo_efetivo: row.get("preco_varejo_efetivo"),
            has_imagem_original: row.get("has_imagem_original"),
            has_imagem_brand: row.get("has_imagem_brand"),
            has_imagem_modelo: row.get("has_imagem_modelo"),
            has_imagem_alternativa: row.get("has_imagem_alternativa"),
        })
        .collect())
}

pub async fn list_estampa_vinculos_by_tecido(
    pool: &PgPool,
    tecido_id: i64,
) -> Result<Vec<VinculoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            te.estampa_id AS cor_id,
            t.nome AS tecido_nome,
            e.nome AS cor_nome,
            NULL::text AS cor_hex,
            te.sku,
            t.custo_base::float8 AS tecido_custo_base,
            te.custo_override::float8 AS custo_override,
            COALESCE(te.custo_override, t.custo_base)::float8 AS custo_efetivo,
            t.preco_atacado::float8 AS tecido_preco_atacado,
            t.preco_varejo::float8 AS tecido_preco_varejo,
            te.preco_atacado_override::float8 AS preco_atacado_override,
            te.preco_varejo_override::float8 AS preco_varejo_override,
            COALESCE(te.preco_atacado_override, t.preco_atacado)::float8 AS preco_atacado_efetivo,
            COALESCE(te.preco_varejo_override, t.preco_varejo)::float8 AS preco_varejo_efetivo,
            te.imagem_original IS NOT NULL AS has_imagem_original,
            te.imagem_brand IS NOT NULL AS has_imagem_brand,
            te.imagem_modelo IS NOT NULL AS has_imagem_modelo,
            te.imagem_alternativa IS NOT NULL AS has_imagem_alternativa
        FROM tecido_estampas te
        JOIN tecidos t ON t.id = te.tecido_id
        JOIN estampas e ON e.id = te.estampa_id
        WHERE te.tecido_id = $1 AND te.ativo = TRUE
        ORDER BY e.nome, e.id
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
            tecido_custo_base: row.get("tecido_custo_base"),
            custo_override: row.get("custo_override"),
            custo_efetivo: row.get("custo_efetivo"),
            tecido_preco_atacado: row.get("tecido_preco_atacado"),
            tecido_preco_varejo: row.get("tecido_preco_varejo"),
            preco_atacado_override: row.get("preco_atacado_override"),
            preco_varejo_override: row.get("preco_varejo_override"),
            preco_atacado_efetivo: row.get("preco_atacado_efetivo"),
            preco_varejo_efetivo: row.get("preco_varejo_efetivo"),
            has_imagem_original: row.get("has_imagem_original"),
            has_imagem_brand: row.get("has_imagem_brand"),
            has_imagem_modelo: row.get("has_imagem_modelo"),
            has_imagem_alternativa: row.get("has_imagem_alternativa"),
        })
        .collect())
}

pub async fn get_vinculo_image(
    pool: &PgPool,
    tecido_id: i64,
    item_id: i64,
    usa_estampas: bool,
    slot: &str,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    let table = if usa_estampas {
        "tecido_estampas"
    } else {
        "tecido_cores"
    };
    let item_column = if usa_estampas { "estampa_id" } else { "cor_id" };
    let column = match slot {
        "original" => "imagem_original",
        "brand" => "imagem_brand",
        "modelo" => "imagem_modelo",
        "alternativa" => "imagem_alternativa",
        _ => "imagem_original",
    };
    let row = sqlx::query(&format!(
        "SELECT {column} AS image FROM {table} WHERE tecido_id = $1 AND {item_column} = $2"
    ))
    .bind(tecido_id)
    .bind(item_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|row| row.get("image")))
}

pub async fn update_vinculo_image(
    pool: &PgPool,
    tecido_id: i64,
    item_id: i64,
    usa_estampas: bool,
    slot: &str,
    bytes: &[u8],
) -> Result<(), sqlx::Error> {
    let table = if usa_estampas {
        "tecido_estampas"
    } else {
        "tecido_cores"
    };
    let item_column = if usa_estampas { "estampa_id" } else { "cor_id" };
    let column = match slot {
        "original" => "imagem_original",
        "brand" => "imagem_brand",
        "modelo" => "imagem_modelo",
        "alternativa" => "imagem_alternativa",
        _ => "imagem_original",
    };
    sqlx::query(&format!(
        "UPDATE {table} SET {column} = $1 WHERE tecido_id = $2 AND {item_column} = $3"
    ))
    .bind(bytes)
    .bind(tecido_id)
    .bind(item_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_vinculo_custo_override(
    pool: &PgPool,
    tecido_id: i64,
    item_id: i64,
    usa_estampas: bool,
    custo_override: Option<f64>,
) -> Result<(), sqlx::Error> {
    let table = if usa_estampas {
        "tecido_estampas"
    } else {
        "tecido_cores"
    };
    let item_column = if usa_estampas { "estampa_id" } else { "cor_id" };

    sqlx::query(&format!(
        "UPDATE {table} SET custo_override = $1::numeric WHERE tecido_id = $2 AND {item_column} = $3"
    ))
    .bind(custo_override)
    .bind(tecido_id)
    .bind(item_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_tecido_preco_venda(
    pool: &PgPool,
    tecido_id: i64,
    tipo: ListaPrecoTipo,
    preco: Option<f64>,
) -> Result<(), sqlx::Error> {
    let column = match tipo {
        ListaPrecoTipo::Custo => "custo_base",
        ListaPrecoTipo::Atacado => "preco_atacado",
        ListaPrecoTipo::Varejo => "preco_varejo",
    };

    sqlx::query(&format!(
        "UPDATE tecidos SET {column} = $1::numeric WHERE id = $2"
    ))
    .bind(preco)
    .bind(tecido_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_vinculo_preco_override(
    pool: &PgPool,
    tecido_id: i64,
    item_id: i64,
    usa_estampas: bool,
    tipo: ListaPrecoTipo,
    preco_override: Option<f64>,
) -> Result<(), sqlx::Error> {
    let table = if usa_estampas {
        "tecido_estampas"
    } else {
        "tecido_cores"
    };
    let item_column = if usa_estampas { "estampa_id" } else { "cor_id" };
    let price_column = match tipo {
        ListaPrecoTipo::Custo => "custo_override",
        ListaPrecoTipo::Atacado => "preco_atacado_override",
        ListaPrecoTipo::Varejo => "preco_varejo_override",
    };

    sqlx::query(&format!(
        "UPDATE {table} SET {price_column} = $1::numeric WHERE tecido_id = $2 AND {item_column} = $3"
    ))
    .bind(preco_override)
    .bind(tecido_id)
    .bind(item_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn deactivate_vinculo(
    pool: &PgPool,
    tecido_id: i64,
    item_id: i64,
    usa_estampas: bool,
) -> Result<(), sqlx::Error> {
    let table = if usa_estampas {
        "tecido_estampas"
    } else {
        "tecido_cores"
    };
    let item_column = if usa_estampas { "estampa_id" } else { "cor_id" };

    sqlx::query(&format!(
        "UPDATE {table} SET ativo = FALSE WHERE tecido_id = $1 AND {item_column} = $2"
    ))
    .bind(tecido_id)
    .bind(item_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn replace_vinculos(
    pool: &PgPool,
    tecido_id: i64,
    vinculos: &[(i64, String)],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;

    let selected_ids = vinculos
        .iter()
        .map(|(cor_id, _)| *cor_id)
        .collect::<Vec<_>>();
    sqlx::query(
        "UPDATE tecido_cores SET ativo = FALSE WHERE tecido_id = $1 AND NOT (cor_id = ANY($2))",
    )
    .bind(tecido_id)
    .bind(&selected_ids)
    .execute(&mut *transaction)
    .await?;

    for (cor_id, sku) in vinculos {
        sqlx::query(
            r#"
            INSERT INTO tecido_cores (tecido_id, cor_id, sku, ativo)
            VALUES ($1, $2, $3, TRUE)
            ON CONFLICT (tecido_id, cor_id)
            DO UPDATE SET sku = EXCLUDED.sku, ativo = TRUE
            "#,
        )
        .bind(tecido_id)
        .bind(cor_id)
        .bind(sku)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;

    Ok(())
}

pub async fn replace_estampa_vinculos(
    pool: &PgPool,
    tecido_id: i64,
    vinculos: &[(i64, String)],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;

    let selected_ids = vinculos
        .iter()
        .map(|(estampa_id, _)| *estampa_id)
        .collect::<Vec<_>>();
    sqlx::query("UPDATE tecido_estampas SET ativo = FALSE WHERE tecido_id = $1 AND NOT (estampa_id = ANY($2))")
        .bind(tecido_id)
        .bind(&selected_ids)
        .execute(&mut *transaction)
        .await?;

    for (estampa_id, sku) in vinculos {
        sqlx::query(
            r#"
            INSERT INTO tecido_estampas (tecido_id, estampa_id, sku, ativo)
            VALUES ($1, $2, $3, TRUE)
            ON CONFLICT (tecido_id, estampa_id)
            DO UPDATE SET sku = EXCLUDED.sku, ativo = TRUE
            "#,
        )
        .bind(tecido_id)
        .bind(estampa_id)
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
