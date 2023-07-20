#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use activitypub_federation::config::FederationConfig;
use ap::objects::user::ApUser;
use dotenvy::dotenv;
use listenfd::ListenFd;
mod api;
mod common;
mod routers;
mod rpc;

mod ap;
mod errors;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

#[derive(Clone)]
pub struct AppState {
    db_pool: Pool<diesel_async::AsyncPgConnection>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init();

    let mut listenfd = ListenFd::from_env();

    let tcp_socket: Option<std::net::TcpListener> = match listenfd.take_tcp_listener(0) {
        Ok(socket) => socket,
        Err(_) => None,
    };

    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(
        std::env::var("DATABASE_URL")?,
    );
    let db_pool = Pool::builder(config).build()?;
    let mut connection = db_pool.get().await?;

    db::migrations::run_migrations(&mut connection).await?; // run all pending migrations

    let state = Arc::new(AppState { db_pool });

    let data = FederationConfig::builder()
        .domain(std::env::var("CRYAP_DOMAIN")?)
        .app_data(Arc::clone(&state))
        .build()
        .await?;

    let rpc_data = Arc::new(data.to_request_data());
    tokio::spawn(async move { rpc::start(rpc_data).await });

    let app = routers::app(data);

    match tcp_socket {
        // cargo-watch thing
        Some(listener) => axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app.into_make_service())
            .await
            .unwrap(),
        None => {
            let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
                .unwrap()
        }
    };

    Ok(())
}
