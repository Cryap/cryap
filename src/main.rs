#![forbid(unsafe_code)]

mod config;
mod frontend;
mod router;

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use activitypub_federation::{
    config::FederationConfig,
    http_signatures::{generate_actor_keypair, Keypair},
};
use ap::objects::service_actor::ServiceActor;
use diesel_async::pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager};
use listenfd::ListenFd;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use tokio_util::task::LocalPoolHandle;
use url::Url;
use web::AppState;

#[derive(Debug, Serialize, Deserialize)]
struct ServiceActorData {
    pubkey: String,
    privkey: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut listenfd = ListenFd::from_env();

    let tcp_socket: Option<std::net::TcpListener> = match listenfd.take_tcp_listener(0) {
        Ok(socket) => socket,
        Err(_) => None,
    };

    let service_actor_keys: Keypair = match std::fs::read_to_string("instance.keys") {
        Ok(str) => {
            let data: ServiceActorData = serde_json::from_str(&str)?;

            Keypair {
                public_key: data.pubkey,
                private_key: data.privkey,
            }
        },
        Err(_) => {
            let keypair = generate_actor_keypair()?;
            let keypair_ = keypair.clone();
            std::fs::write(
                "instance.keys",
                serde_json::to_string(&ServiceActorData {
                    privkey: keypair_.private_key,
                    pubkey: keypair_.public_key,
                })?,
            )?;
            keypair
        },
    };

    let config = match config::process_config() {
        Ok(config) => config,
        Err(err) => {
            log::error!("Can't parse config.toml :(");
            return Err(err);
        },
    };

    let db_config =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(&config.database.uri);
    let db_pool = Pool::builder(db_config).build()?;
    let mut connection = db_pool.get().await?;

    db::migrations::run_migrations(&mut connection).await?; // run all pending migrations

    let service_actor = ServiceActor::new(
        Url::parse(&format!("https://{}/ap/actor", &config.web.domain))?,
        service_actor_keys,
    );

    let redis_client = redis::Client::open(config.redis.uri.clone())?;
    let state = Arc::new(AppState {
        db_pool,
        redis: ConnectionManager::new(redis_client).await?,
        config,
        local_pool: LocalPoolHandle::new(20),
    });

    let data = FederationConfig::builder()
        .domain(&state.config.web.domain)
        .app_data(Arc::clone(&state))
        .http_signature_compat(true) // Pleroma federation
        .signed_fetch_actor(&service_actor)
        .build()
        .await?;

    let rpc_data = Arc::new(data.to_request_data());
    tokio::spawn(async move { rpc::start(rpc_data).await });

    let app = router::app(data, service_actor.clone());

    match tcp_socket {
        // cargo-watch thing
        Some(listener) => axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app.into_make_service())
            .await
            .unwrap(),
        None => {
            let addr = SocketAddr::from((
                state.config.web.host.parse::<IpAddr>()?,
                state.config.web.port,
            ));
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
                .unwrap()
        },
    };

    Ok(())
}
