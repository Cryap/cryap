use std::sync::Arc;

use activitypub_federation::config::Data;
use axum::{response::IntoResponse, routing::get, Json, Router};
use db::common::nodeinfo::get_nodeinfo_usage;
use url::Url;
use web::{errors::AppError, AppState};

use crate::common::nodeinfo::{
    NodeInfo, NodeInfoMetadata, NodeInfoServices, NodeInfoSoftware, NodeInfoSoftware2_0,
    NodeInfoSoftware2_1, NodeInfoWellKnown, NodeInfoWellKnownLinks,
};

async fn get_base_nodeinfo(state: &Data<Arc<AppState>>) -> anyhow::Result<NodeInfo> {
    Ok(NodeInfo {
        version: None,
        software: None,
        protocols: Some(vec!["activitypub".to_string()]),
        usage: Some(get_nodeinfo_usage(&state.db_pool).await?),
        open_registrations: Some(false),
        services: Some(NodeInfoServices {
            inbound: Some(vec![]),
            outbound: Some(vec![]),
        }),
        metadata: Some(NodeInfoMetadata {
            node_name: Some(state.config.instance.title.clone()),
            node_description: Some(state.config.instance.description.clone()),
        }),
    })
}

pub async fn http_get_well_known_nodeinfo(
    state: Data<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(NodeInfoWellKnown {
        links: vec![
            NodeInfoWellKnownLinks {
                rel: Url::parse("http://nodeinfo.diaspora.software/ns/schema/2.0")?,
                href: Url::parse(&format!("https://{}/nodeinfo/2.0", state.config.web.domain))?,
            },
            NodeInfoWellKnownLinks {
                rel: Url::parse("http://nodeinfo.diaspora.software/ns/schema/2.1")?,
                href: Url::parse(&format!("https://{}/nodeinfo/2.1", state.config.web.domain))?,
            },
        ],
    }))
}

pub async fn http_get_nodeinfo_2_0(
    state: Data<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let mut nodeinfo = get_base_nodeinfo(&state).await?;
    nodeinfo.version = Some("2.0".to_string());
    nodeinfo.software = Some(NodeInfoSoftware::Version2_0(NodeInfoSoftware2_0 {
        name: Some("cryap".to_string()),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }));
    Ok(Json(nodeinfo))
}

pub async fn http_get_nodeinfo_2_1(
    state: Data<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let mut nodeinfo = get_base_nodeinfo(&state).await?;
    nodeinfo.version = Some("2.1".to_string());
    nodeinfo.software = Some(NodeInfoSoftware::Version2_1(NodeInfoSoftware2_1 {
        name: Some("cryap".to_string()),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        repository: Some(env!("CARGO_PKG_REPOSITORY").to_string()),
        homepage: Some(env!("CARGO_PKG_HOMEPAGE").to_string()),
    }));
    Ok(Json(nodeinfo))
}

pub fn nodeinfo() -> Router {
    Router::new()
        .route("/.well-known/nodeinfo", get(http_get_well_known_nodeinfo))
        .route("/nodeinfo/2.0.json", get(http_get_nodeinfo_2_0))
        .route("/nodeinfo/2.1.json", get(http_get_nodeinfo_2_1))
}
