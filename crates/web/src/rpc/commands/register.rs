use std::sync::Arc;

use activitypub_federation::config::Data;
use serde::{Deserialize, Serialize};

use crate::{common::users, AppState};

#[derive(Deserialize, Debug)]
pub struct RpcRegisterUserData {
    name: String,
    password: String,
    bio: Option<String>,
    display_name: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct RpcRegisterUserResponse {
    ok: bool,
}

pub struct RpcRegisterUser;

impl RpcRegisterUser {
    pub async fn call(
        request: RpcRegisterUserData,
        data: &Data<Arc<AppState>>,
    ) -> RpcRegisterUserResponse {
        let _ = users::register(
            request.name,
            request.password,
            request.bio,
            request.display_name,
            data,
        )
        .await;

        RpcRegisterUserResponse { ok: true }
    }
}
