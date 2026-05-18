use sqlx::{PgPool, Row};

use crate::models::VendaItem;

#[derive(Clone)]
pub struct VendaHistoricoRecord {
    pub id: i64,
    pub created_at: String,
    pub total: f64,
    pub itens: i64,
}

pub async fn insert_venda(pool: &PgPool, itens: &[VendaItem]) -> Result<i64, sqlx::Error> {
    let total = itens.iter().map(VendaItem::total).sum::<f64>();
    let mut transaction = pool.begin().await?;

    let row = sqlx::query("INSERT INTO vendas (total) VALUES ($1::numeric) RETURNING id")
        .bind(total)
        .fetch_one(&mut *transaction)
        .await?;
    let venda_id: i64 = row.get("id");

    insert_venda_itens(&mut transaction, venda_id, itens).await?;
    transaction.commit().await?;
    Ok(venda_id)
}

pub async fn list_vendas(pool: &PgPool) -> Result<Vec<VendaHistoricoRecord>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            v.id,
            to_char(v.created_at AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS created_at,
            v.total::float8 AS total,
            COUNT(vi.id)::bigint AS itens
        FROM vendas v
        LEFT JOIN venda_itens vi ON vi.venda_id = v.id
        GROUP BY v.id, v.created_at, v.total
        ORDER BY v.created_at DESC, v.id DESC
        LIMIT 100
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| VendaHistoricoRecord {
            id: row.get("id"),
            created_at: row.get("created_at"),
            total: row.get("total"),
            itens: row.get("itens"),
        })
        .collect())
}

pub async fn update_venda(
    pool: &PgPool,
    venda_id: i64,
    itens: &[VendaItem],
) -> Result<(), sqlx::Error> {
    let total = itens.iter().map(VendaItem::total).sum::<f64>();
    let mut transaction = pool.begin().await?;

    sqlx::query("UPDATE vendas SET total = $1::numeric WHERE id = $2")
        .bind(total)
        .bind(venda_id)
        .execute(&mut *transaction)
        .await?;

    sqlx::query("DELETE FROM venda_itens WHERE venda_id = $1")
        .bind(venda_id)
        .execute(&mut *transaction)
        .await?;

    insert_venda_itens(&mut transaction, venda_id, itens).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn delete_venda(pool: &PgPool, venda_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM vendas WHERE id = $1")
        .bind(venda_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_venda_itens(pool: &PgPool, venda_id: i64) -> Result<Vec<VendaItem>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT descricao, quantidade::float8 AS quantidade, preco_unitario::float8 AS preco_unitario
        FROM venda_itens
        WHERE venda_id = $1
        ORDER BY id
        "#,
    )
    .bind(venda_id)
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

async fn insert_venda_itens(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    venda_id: i64,
    itens: &[VendaItem],
) -> Result<(), sqlx::Error> {
    for item in itens {
        sqlx::query(
            r#"
            INSERT INTO venda_itens (
                venda_id,
                descricao,
                quantidade,
                preco_unitario,
                subtotal
            )
            VALUES ($1, $2, $3::numeric, $4::numeric, $5::numeric)
            "#,
        )
        .bind(venda_id)
        .bind(&item.descricao)
        .bind(item.quantidade)
        .bind(item.preco_unitario)
        .bind(item.total())
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}
