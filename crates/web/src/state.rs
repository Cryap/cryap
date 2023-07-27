use diesel_async::pooled_connection::deadpool::Pool;
use redis::aio::ConnectionManager;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool<diesel_async::AsyncPgConnection>,
    pub redis: ConnectionManager,
}
