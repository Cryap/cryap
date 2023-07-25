use diesel_async::pooled_connection::deadpool::Pool;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool<diesel_async::AsyncPgConnection>,
}
