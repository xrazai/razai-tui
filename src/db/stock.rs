use std::collections::BTreeMap;

use chrono::NaiveDate;
use sqlx::{PgPool, Row};

use crate::models::VendaItem;

#[derive(Clone)]
pub struct EstoqueSaldoRecord {
    pub tecido_id: i64,
    pub item_id: i64,
    pub usa_estampas: bool,
    pub sku: Option<String>,
    pub tecido_nome: String,
    pub item_nome: String,
    pub saldo: f64,
    pub preco_atacado: Option<f64>,
    pub preco_varejo: Option<f64>,
}

#[derive(Clone)]
pub struct EstoqueMovimentoRecord {
    pub created_at: String,
    pub tipo: String,
    pub quantidade: f64,
    pub venda_id: Option<i64>,
    pub destino: Option<String>,
    pub observacao: Option<String>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct EstoqueOrdemRecord {
    pub id: i64,
    pub created_at: String,
    pub tecido_id: i64,
    pub item_id: i64,
    pub usa_estampas: bool,
    pub sku: Option<String>,
    pub tecido_nome: String,
    pub item_nome: String,
    pub quantidade: f64,
    pub status: String,
    pub fornecedor_id: Option<i64>,
    pub fornecedor_nome: Option<String>,
    pub venda_id: Option<i64>,
    pub observacao: Option<String>,
}

#[derive(Clone)]
pub struct FornecedorResumoVendaRecord {
    pub tecido_nome: String,
    pub quantidade: f64,
    pub custo_total: f64,
}

#[derive(Clone)]
pub struct MaisVendidoRecord {
    pub tecido_nome: String,
    pub item_nome: String,
    pub sku: Option<String>,
    pub quantidade: f64,
}

pub async fn ensure_estoque_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('razai_estoque_schema'))")
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS estoque_movimentacoes (
            id BIGSERIAL PRIMARY KEY,
            tecido_id BIGINT NOT NULL REFERENCES tecidos(id) ON DELETE CASCADE,
            item_id BIGINT NOT NULL,
            usa_estampas BOOLEAN NOT NULL DEFAULT FALSE,
            tipo TEXT NOT NULL,
            quantidade NUMERIC(12, 3) NOT NULL,
            venda_id BIGINT REFERENCES vendas(id) ON DELETE CASCADE,
            destino TEXT,
            observacao TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_estoque_mov_venda_id ON estoque_movimentacoes(venda_id)",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_estoque_mov_vinculo ON estoque_movimentacoes(tecido_id, item_id, usa_estampas)",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS estoque_ordens (
            id BIGSERIAL PRIMARY KEY,
            tecido_id BIGINT NOT NULL REFERENCES tecidos(id) ON DELETE CASCADE,
            item_id BIGINT NOT NULL,
            usa_estampas BOOLEAN NOT NULL DEFAULT FALSE,
            quantidade NUMERIC(12, 3) NOT NULL,
            status TEXT NOT NULL DEFAULT 'pendente',
            fornecedor_id BIGINT REFERENCES fornecedores(id) ON DELETE SET NULL,
            venda_id BIGINT REFERENCES vendas(id) ON DELETE CASCADE,
            observacao TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_estoque_ordens_status ON estoque_ordens(status)")
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_estoque_ordens_venda_id ON estoque_ordens(venda_id)",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_estoque_ordens_vinculo ON estoque_ordens(tecido_id, item_id, usa_estampas)",
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn list_estoque_saldos(pool: &PgPool) -> Result<Vec<EstoqueSaldoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            tc.tecido_id,
            tc.cor_id AS item_id,
            FALSE AS usa_estampas,
            tc.sku,
            t.nome AS tecido_nome,
            CASE WHEN tc.ativo THEN c.nome ELSE c.nome || ' (inativo)' END AS item_nome,
            COALESCE(SUM(em.quantidade), 0)::float8 AS saldo,
            COALESCE(tc.preco_atacado_override, t.preco_atacado)::float8 AS preco_atacado,
            COALESCE(tc.preco_varejo_override, t.preco_varejo)::float8 AS preco_varejo
        FROM tecido_cores tc
        JOIN tecidos t ON t.id = tc.tecido_id
        JOIN cores c ON c.id = tc.cor_id
        LEFT JOIN estoque_movimentacoes em
            ON em.tecido_id = tc.tecido_id
            AND em.item_id = tc.cor_id
            AND em.usa_estampas = FALSE
        WHERE tc.ativo = TRUE OR em.id IS NOT NULL
        GROUP BY tc.tecido_id, tc.cor_id, tc.sku, t.nome, c.nome, tc.ativo, tc.preco_atacado_override, tc.preco_varejo_override, t.preco_atacado, t.preco_varejo
        UNION ALL
        SELECT
            te.tecido_id,
            te.estampa_id AS item_id,
            TRUE AS usa_estampas,
            te.sku,
            t.nome AS tecido_nome,
            CASE WHEN te.ativo THEN e.nome ELSE e.nome || ' (inativo)' END AS item_nome,
            COALESCE(SUM(em.quantidade), 0)::float8 AS saldo,
            COALESCE(te.preco_atacado_override, t.preco_atacado)::float8 AS preco_atacado,
            COALESCE(te.preco_varejo_override, t.preco_varejo)::float8 AS preco_varejo
        FROM tecido_estampas te
        JOIN tecidos t ON t.id = te.tecido_id
        JOIN estampas e ON e.id = te.estampa_id
        LEFT JOIN estoque_movimentacoes em
            ON em.tecido_id = te.tecido_id
            AND em.item_id = te.estampa_id
            AND em.usa_estampas = TRUE
        WHERE te.ativo = TRUE OR em.id IS NOT NULL
        GROUP BY te.tecido_id, te.estampa_id, te.sku, t.nome, e.nome, te.ativo, te.preco_atacado_override, te.preco_varejo_override, t.preco_atacado, t.preco_varejo
        ORDER BY tecido_nome, item_nome
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_saldo).collect())
}

pub async fn list_estoque_movimentos(
    pool: &PgPool,
    tecido_id: i64,
    item_id: i64,
    usa_estampas: bool,
) -> Result<Vec<EstoqueMovimentoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            to_char(created_at AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS created_at,
            tipo,
            quantidade::float8 AS quantidade,
            venda_id,
            destino,
            observacao
        FROM estoque_movimentacoes
        WHERE tecido_id = $1 AND item_id = $2 AND usa_estampas = $3
        ORDER BY created_at DESC, id DESC
        LIMIT 100
        "#,
    )
    .bind(tecido_id)
    .bind(item_id)
    .bind(usa_estampas)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| EstoqueMovimentoRecord {
            created_at: row.get("created_at"),
            tipo: row.get("tipo"),
            quantidade: row.get("quantidade"),
            venda_id: row.get("venda_id"),
            destino: row.get("destino"),
            observacao: row.get("observacao"),
        })
        .collect())
}

pub async fn list_estoque_ordens(pool: &PgPool) -> Result<Vec<EstoqueOrdemRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            MIN(eo.id) AS id,
            to_char(MIN(eo.created_at) AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS created_at,
            eo.tecido_id,
            eo.item_id,
            eo.usa_estampas,
            CASE WHEN eo.usa_estampas THEN te.sku ELSE tc.sku END AS sku,
            t.nome AS tecido_nome,
            COALESCE(CASE WHEN eo.usa_estampas THEN e.nome ELSE c.nome END, 'item removido') AS item_nome,
            SUM(eo.quantidade)::float8 AS quantidade,
            eo.status,
            eo.fornecedor_id,
            f.nome AS fornecedor_nome,
            eo.venda_id,
            STRING_AGG(DISTINCT eo.observacao, ' ') FILTER (WHERE eo.observacao IS NOT NULL AND eo.observacao <> '') AS observacao
        FROM estoque_ordens eo
        JOIN tecidos t ON t.id = eo.tecido_id
        LEFT JOIN tecido_cores tc
            ON tc.tecido_id = eo.tecido_id
            AND tc.cor_id = eo.item_id
            AND eo.usa_estampas = FALSE
        LEFT JOIN cores c ON c.id = tc.cor_id
        LEFT JOIN tecido_estampas te
            ON te.tecido_id = eo.tecido_id
            AND te.estampa_id = eo.item_id
            AND eo.usa_estampas = TRUE
        LEFT JOIN estampas e ON e.id = te.estampa_id
        LEFT JOIN fornecedores f ON f.id = eo.fornecedor_id
        GROUP BY
            eo.venda_id,
            eo.tecido_id,
            eo.item_id,
            eo.usa_estampas,
            CASE WHEN eo.usa_estampas THEN te.sku ELSE tc.sku END,
            t.nome,
            COALESCE(CASE WHEN eo.usa_estampas THEN e.nome ELSE c.nome END, 'item removido'),
            eo.status,
            eo.fornecedor_id,
            f.nome
        ORDER BY
            CASE eo.status
                WHEN 'pendente' THEN 0
                WHEN 'direcionada' THEN 1
                WHEN 'concluida' THEN 2
                ELSE 3
            END,
            MIN(eo.created_at) DESC,
            MIN(eo.id) DESC
        LIMIT 200
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| EstoqueOrdemRecord {
            id: row.get("id"),
            created_at: row.get("created_at"),
            tecido_id: row.get("tecido_id"),
            item_id: row.get("item_id"),
            usa_estampas: row.get("usa_estampas"),
            sku: row.get("sku"),
            tecido_nome: row.get("tecido_nome"),
            item_nome: row.get("item_nome"),
            quantidade: row.get("quantidade"),
            status: row.get("status"),
            fornecedor_id: row.get("fornecedor_id"),
            fornecedor_nome: row.get("fornecedor_nome"),
            venda_id: row.get("venda_id"),
            observacao: row.get("observacao"),
        })
        .collect())
}

pub async fn direcionar_estoque_ordem(
    pool: &PgPool,
    ordem_id: i64,
    fornecedor_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        WITH alvo AS (
            SELECT venda_id, tecido_id, item_id, usa_estampas, status
            FROM estoque_ordens
            WHERE id = $2 AND status IN ('pendente', 'direcionada')
        )
        UPDATE estoque_ordens eo
        SET fornecedor_id = $1, status = 'direcionada', updated_at = NOW()
        FROM alvo
        WHERE eo.status = alvo.status
            AND eo.tecido_id = alvo.tecido_id
            AND eo.item_id = alvo.item_id
            AND eo.usa_estampas = alvo.usa_estampas
            AND eo.venda_id IS NOT DISTINCT FROM alvo.venda_id
        "#,
    )
    .bind(fornecedor_id)
    .bind(ordem_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_estoque_ordem_status(
    pool: &PgPool,
    ordem_id: i64,
    status: &str,
) -> Result<bool, sqlx::Error> {
    if !matches!(status, "concluida" | "cancelada") {
        return Ok(false);
    }
    let result = sqlx::query(
        r#"
        WITH alvo AS (
            SELECT venda_id, tecido_id, item_id, usa_estampas, status
            FROM estoque_ordens
            WHERE id = $2 AND status IN ('pendente', 'direcionada')
        )
        UPDATE estoque_ordens eo
        SET status = $1, updated_at = NOW()
        FROM alvo
        WHERE eo.status = alvo.status
            AND eo.tecido_id = alvo.tecido_id
            AND eo.item_id = alvo.item_id
            AND eo.usa_estampas = alvo.usa_estampas
            AND eo.venda_id IS NOT DISTINCT FROM alvo.venda_id
        "#,
    )
    .bind(status)
    .bind(ordem_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_fornecedor_resumo_vendas(
    pool: &PgPool,
    fornecedor_id: i64,
    inicio: NaiveDate,
    fim: NaiveDate,
) -> Result<Vec<FornecedorResumoVendaRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            t.nome AS tecido_nome,
            SUM(vi.quantidade)::float8 AS quantidade,
            SUM(
                vi.quantidade * COALESCE(tc.custo_override, te.custo_override, t.custo_base, 0)
            )::float8 AS custo_total
        FROM venda_itens vi
        JOIN vendas v ON v.id = vi.venda_id
        JOIN tecidos t ON t.id = vi.estoque_tecido_id
        LEFT JOIN tecido_cores tc
            ON tc.tecido_id = vi.estoque_tecido_id
            AND tc.cor_id = vi.estoque_item_id
            AND vi.estoque_usa_estampas = FALSE
        LEFT JOIN tecido_estampas te
            ON te.tecido_id = vi.estoque_tecido_id
            AND te.estampa_id = vi.estoque_item_id
            AND vi.estoque_usa_estampas = TRUE
        WHERE (
                t.fornecedor_id = $1
                OR EXISTS (
                    SELECT 1
                    FROM estoque_ordens eo
                    WHERE eo.fornecedor_id = $1
                        AND eo.venda_id = vi.venda_id
                        AND eo.tecido_id = vi.estoque_tecido_id
                        AND eo.item_id = vi.estoque_item_id
                        AND eo.usa_estampas = vi.estoque_usa_estampas
                )
            )
            AND (v.created_at AT TIME ZONE 'America/Sao_Paulo')::date BETWEEN $2 AND $3
        GROUP BY t.id, t.nome
        ORDER BY t.nome
        "#,
    )
    .bind(fornecedor_id)
    .bind(inicio)
    .bind(fim)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| FornecedorResumoVendaRecord {
            tecido_nome: row.get("tecido_nome"),
            quantidade: row.get("quantidade"),
            custo_total: row.get("custo_total"),
        })
        .collect())
}

pub async fn list_mais_vendidos(pool: &PgPool) -> Result<Vec<MaisVendidoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            t.nome AS tecido_nome,
            COALESCE(CASE WHEN vi.estoque_usa_estampas THEN e.nome ELSE c.nome END, 'item removido') AS item_nome,
            CASE WHEN vi.estoque_usa_estampas THEN te.sku ELSE tc.sku END AS sku,
            SUM(vi.quantidade)::float8 AS quantidade
        FROM venda_itens vi
        JOIN vendas v ON v.id = vi.venda_id
        JOIN tecidos t ON t.id = vi.estoque_tecido_id
        LEFT JOIN tecido_cores tc
            ON tc.tecido_id = vi.estoque_tecido_id
            AND tc.cor_id = vi.estoque_item_id
            AND vi.estoque_usa_estampas = FALSE
        LEFT JOIN cores c ON c.id = tc.cor_id
        LEFT JOIN tecido_estampas te
            ON te.tecido_id = vi.estoque_tecido_id
            AND te.estampa_id = vi.estoque_item_id
            AND vi.estoque_usa_estampas = TRUE
        LEFT JOIN estampas e ON e.id = te.estampa_id
        WHERE vi.estoque_tecido_id IS NOT NULL
            AND vi.estoque_item_id IS NOT NULL
        GROUP BY
            t.nome,
            COALESCE(CASE WHEN vi.estoque_usa_estampas THEN e.nome ELSE c.nome END, 'item removido'),
            CASE WHEN vi.estoque_usa_estampas THEN te.sku ELSE tc.sku END
        ORDER BY quantidade DESC, t.nome, item_nome
        LIMIT 100
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| MaisVendidoRecord {
            tecido_nome: row.get("tecido_nome"),
            item_nome: row.get("item_nome"),
            sku: row.get("sku"),
            quantidade: row.get("quantidade"),
        })
        .collect())
}

pub async fn insert_estoque_manual(
    pool: &PgPool,
    tecido_id: i64,
    item_id: i64,
    usa_estampas: bool,
    tipo: &str,
    quantidade_abs: f64,
    destino: Option<&str>,
    observacao: Option<&str>,
) -> Result<(), sqlx::Error> {
    let quantidade = if tipo == "entrada" {
        quantidade_abs
    } else {
        -quantidade_abs
    };
    sqlx::query(
        r#"
        INSERT INTO estoque_movimentacoes
            (tecido_id, item_id, usa_estampas, tipo, quantidade, destino, observacao)
        VALUES ($1, $2, $3, $4, $5::numeric, $6, $7)
        "#,
    )
    .bind(tecido_id)
    .bind(item_id)
    .bind(usa_estampas)
    .bind(tipo)
    .bind(quantidade)
    .bind(destino.map(str::trim).filter(|value| !value.is_empty()))
    .bind(observacao.map(str::trim).filter(|value| !value.is_empty()))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn reset_sale_stock_movements(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    venda_id: i64,
    itens: &[VendaItem],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM estoque_movimentacoes WHERE venda_id = $1")
        .bind(venda_id)
        .execute(&mut **transaction)
        .await?;
    sqlx::query("DELETE FROM estoque_ordens WHERE venda_id = $1 AND status = 'pendente'")
        .bind(venda_id)
        .execute(&mut **transaction)
        .await?;
    sqlx::query(
        r#"
        UPDATE estoque_ordens
        SET
            status = 'cancelada',
            observacao = CONCAT_WS(' ', NULLIF(observacao, ''), 'Cancelada automaticamente ao recalcular a venda.'),
            updated_at = NOW()
        WHERE venda_id = $1 AND status = 'direcionada'
        "#,
    )
    .bind(venda_id)
    .execute(&mut **transaction)
    .await?;

    let mut grouped_items = BTreeMap::<(i64, i64, bool), f64>::new();
    for item in itens {
        let (Some(tecido_id), Some(item_id)) = (item.estoque_tecido_id, item.estoque_item_id)
        else {
            continue;
        };
        *grouped_items
            .entry((tecido_id, item_id, item.estoque_usa_estampas))
            .or_insert(0.0) += item.quantidade;
    }

    for ((tecido_id, item_id, usa_estampas), quantidade) in grouped_items {
        let saldo_antes: f64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(quantidade), 0)::float8
            FROM estoque_movimentacoes
            WHERE tecido_id = $1 AND item_id = $2 AND usa_estampas = $3
            "#,
        )
        .bind(tecido_id)
        .bind(item_id)
        .bind(usa_estampas)
        .fetch_one(&mut **transaction)
        .await?;
        let faltante = if saldo_antes <= 0.0 {
            quantidade
        } else {
            (quantidade - saldo_antes).max(0.0)
        };
        if faltante > 0.0 {
            sqlx::query(
                r#"
                INSERT INTO estoque_ordens
                    (tecido_id, item_id, usa_estampas, quantidade, status, venda_id, observacao)
                VALUES ($1, $2, $3, $4::numeric, 'pendente', $5, $6)
                "#,
            )
            .bind(tecido_id)
            .bind(item_id)
            .bind(usa_estampas)
            .bind(faltante)
            .bind(venda_id)
            .bind("Criada automaticamente por falta de estoque na venda.")
            .execute(&mut **transaction)
            .await?;
        }
        sqlx::query(
            r#"
            INSERT INTO estoque_movimentacoes
                (tecido_id, item_id, usa_estampas, tipo, quantidade, venda_id)
            VALUES ($1, $2, $3, 'saida_venda', $4::numeric, $5)
            "#,
        )
        .bind(tecido_id)
        .bind(item_id)
        .bind(usa_estampas)
        .bind(-quantidade)
        .bind(venda_id)
        .execute(&mut **transaction)
        .await?;
    }

    Ok(())
}

fn map_saldo(row: sqlx::postgres::PgRow) -> EstoqueSaldoRecord {
    EstoqueSaldoRecord {
        tecido_id: row.get("tecido_id"),
        item_id: row.get("item_id"),
        usa_estampas: row.get("usa_estampas"),
        sku: row.get("sku"),
        tecido_nome: row.get("tecido_nome"),
        item_nome: row.get("item_nome"),
        saldo: row.get("saldo"),
        preco_atacado: row.get("preco_atacado"),
        preco_varejo: row.get("preco_varejo"),
    }
}
