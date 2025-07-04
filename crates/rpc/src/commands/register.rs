use std::sync::Arc;

use activitypub_federation::config::Data;
use api::common::users;
use serde::{Deserialize, Serialize};
use web::AppState;

#[derive(Deserialize, Debug)]
pub(crate) struct RpcRegisterUserData {
    name: String,
    password: String,
    bio: Option<String>,
    display_name: Option<String>,
}

#[derive(Serialize, Debug)]
pub(crate) struct RpcRegisterUserResponse {
    ok: bool,
}

pub(crate) struct RpcRegisterUser;

impl RpcRegisterUser {
    pub(crate) async fn call(
        request: RpcRegisterUserData,
        data: &Data<Arc<AppState>>,
    ) -> RpcRegisterUserResponse {
        let user = users::register(
            request.name,
            request.password,
            request.bio,
            request.display_name,
            data,
        )
        .await;
        match user {
            Ok(_) => RpcRegisterUserResponse { ok: true },
            Err(err) => {
                log::error!("Error from RPC command, {:#?}", err);
                RpcRegisterUserResponse { ok: false }
            },
        }
    }
}
