use sqlx::{PgPool, Row};

use crate::models::VendaItem;

#[derive(Clone)]
pub struct PedidoRecord {
    pub id: i64,
    pub created_at: String,
    pub total: f64,
    pub itens: i64,
    pub status: String,
    pub pdf_path: Option<String>,
}

pub async fn insert_pedido(
    pool: &PgPool,
    itens: &[VendaItem],
    pdf_path: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let total = itens.iter().map(VendaItem::total).sum::<f64>();
    let mut transaction = pool.begin().await?;

    let row = sqlx::query(
        "INSERT INTO pedidos (total, status, pdf_path) VALUES ($1::numeric, 'pendente', $2) RETURNING id",
    )
    .bind(total)
    .bind(pdf_path)
    .fetch_one(&mut *transaction)
    .await?;
    let pedido_id: i64 = row.get("id");

    insert_pedido_itens(&mut transaction, pedido_id, itens).await?;
    transaction.commit().await?;
    Ok(pedido_id)
}

pub async fn list_pedidos(pool: &PgPool) -> Result<Vec<PedidoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            p.id,
            to_char(p.created_at AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS created_at,
            p.total::float8 AS total,
            p.status,
            p.pdf_path,
            COUNT(pi.id)::bigint AS itens
        FROM pedidos p
        LEFT JOIN pedido_itens pi ON pi.pedido_id = p.id
        GROUP BY p.id, p.created_at, p.total, p.status, p.pdf_path
        ORDER BY p.created_at DESC, p.id DESC
        LIMIT 100
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PedidoRecord {
            id: row.get("id"),
            created_at: row.get("created_at"),
            total: row.get("total"),
            status: row.get("status"),
            pdf_path: row.get("pdf_path"),
            itens: row.get("itens"),
        })
        .collect())
}

pub async fn list_pedido_itens(
    pool: &PgPool,
    pedido_id: i64,
) -> Result<Vec<VendaItem>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT descricao, quantidade::float8 AS quantidade, preco_unitario::float8 AS preco_unitario
        FROM pedido_itens
        WHERE pedido_id = $1
        ORDER BY id
        "#,
    )
    .bind(pedido_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| VendaItem {
            descricao: row.get("descricao"),
            quantidade: row.get("quantidade"),
            preco_unitario: row.get("preco_unitario"),
        })
        .collect())
}

pub async fn approve_pedido(pool: &PgPool, pedido_id: i64) -> Result<(), sqlx::Error> {
    let itens = list_pedido_itens(pool, pedido_id).await?;
    crate::db::insert_venda(pool, &itens).await?;
    sqlx::query("UPDATE pedidos SET status = 'aprovado' WHERE id = $1")
        .bind(pedido_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_pedido_pdf_path(
    pool: &PgPool,
    pedido_id: i64,
    pdf_path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE pedidos SET pdf_path = $1 WHERE id = $2")
        .bind(pdf_path)
        .bind(pedido_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn insert_pedido_itens(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    pedido_id: i64,
    itens: &[VendaItem],
) -> Result<(), sqlx::Error> {
    for item in itens {
        sqlx::query(
            r#"
            INSERT INTO pedido_itens (
                pedido_id,
                descricao,
                quantidade,
                preco_unitario,
                subtotal
            )
            VALUES ($1, $2, $3::numeric, $4::numeric, $5::numeric)
            "#,
        )
        .bind(pedido_id)
        .bind(&item.descricao)
        .bind(item.quantidade)
        .bind(item.preco_unitario)
        .bind(item.total())
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}
