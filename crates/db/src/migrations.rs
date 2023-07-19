use diesel_async::pg;
use lazy_static::lazy_static;

lazy_static! {
    static ref MIGRATIONS: diesel_async_migrations::EmbeddedMigrations =
        diesel_async_migrations::embed_migrations!();
}

pub async fn run_migrations(conn: &mut pg::AsyncPgConnection) -> anyhow::Result<()> {
    MIGRATIONS.run_pending_migrations(conn).await?;
    Ok(())
}
