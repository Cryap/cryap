use std::sync::Arc;

use activitypub_federation::{config::Data, fetch::webfinger::webfinger_resolve_actor};
use ap::objects::user::ApUser;
use serde::Serialize;
use web::AppState;

#[derive(Serialize, Debug)]
pub(crate) struct RpcUserFetchResponse {
    ok: bool,
}

pub(crate) struct RpcUserFetch;

impl RpcUserFetch {
    pub(crate) async fn call(request: String, data: &Data<Arc<AppState>>) -> RpcUserFetchResponse {
        let user = webfinger_resolve_actor::<Arc<AppState>, ApUser>(&request, data).await;
        match user {
            Ok(_) => RpcUserFetchResponse { ok: true },
            Err(err) => {
                log::error!("Error from RPC command, {:#?}", err);
                RpcUserFetchResponse { ok: false }
            },
        }
    }
}
