use diesel_async::pooled_connection::deadpool::Pool;
use redis::aio::ConnectionManager;
use tokio_util::task::LocalPoolHandle;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool<diesel_async::AsyncPgConnection>,
    pub redis: ConnectionManager,
    pub config: Config,
    pub local_pool: LocalPoolHandle,
}
