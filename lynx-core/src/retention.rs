use log::{info, warn};
use sqlx::PgPool;

pub async fn prune_old_metrics(pool: &PgPool, older_than_days: i64) -> Result<(), sqlx::Error> {
    if older_than_days <= 0 {
        return Ok(());
    }

    let tables: &[(&str, &str)] = &[
        ("gpu_metrics", "time"),
        ("container_metrics", "time"),
        ("metrics", "time"),
    ];

    const BATCH_LIMIT: i64 = 10_000;
    let mut total_deleted: i64 = 0;

    for (table, col) in tables {
        let mut table_deleted: i64 = 0;

        loop {
            let sql = format!(
                "WITH c AS (
                    SELECT ctid FROM {table}
                    WHERE {col} < NOW() - ($1 * INTERVAL '1 day')
                    LIMIT {batch}
                 )
                 DELETE FROM {table} t
                 USING c
                 WHERE t.ctid = c.ctid",
                table = table,
                col = col,
                batch = BATCH_LIMIT
            );

            let res = sqlx::query(&sql)
                .bind(older_than_days)
                .execute(pool)
                .await?;
            let affected = res.rows_affected() as i64;

            if affected == 0 {
                break;
            }
            table_deleted += affected;
            total_deleted += affected;

            if affected < BATCH_LIMIT {
                break;
            }
        }

        if table_deleted > 0 {
            info!("[retention] Pruned {table_deleted} rows from {table}");
        }
    }

    if total_deleted == 0 {
        warn!(
            "[retention] No metric rows to prune (>{} days)",
            older_than_days
        );
    } else {
        info!(
            "[retention] Total pruned rows: {} (older than {} days)",
            total_deleted, older_than_days
        );
    }

    Ok(())
}
