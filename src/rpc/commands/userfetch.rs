use std::sync::Arc;

use activitypub_federation::{config::Data, fetch::webfinger::webfinger_resolve_actor};
use serde::Serialize;

use crate::{ApUser, AppState};

#[derive(Serialize, Debug)]
pub struct RpcUserFetchResponse {
    ok: bool,
}

pub struct RpcUserFetch;

impl RpcUserFetch {
    pub async fn call(request: String, data: &Data<Arc<AppState>>) -> RpcUserFetchResponse {
        let user = webfinger_resolve_actor::<Arc<AppState>, ApUser>(&request, data).await;
        match user {
            Ok(_) => RpcUserFetchResponse { ok: true },
            Err(err) => {
                log::error!("Error from RPC command, {:#?}", err);
                RpcUserFetchResponse { ok: false }
            }
        }
    }
}
