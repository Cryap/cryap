pub(crate) mod register;
pub(crate) mod userfetch;

use serde::{Deserialize, Serialize};

use crate::commands::{
    register::{RpcRegisterUserData, RpcRegisterUserResponse},
    userfetch::RpcUserFetchResponse,
};

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "content")]
pub(crate) enum RpcCommandData {
    UserFetch(String),
    RegisterUser(RpcRegisterUserData),
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "content")]
pub(crate) enum RpcCommandResponse {
    UserFetch(RpcUserFetchResponse),
    RegisterUser(RpcRegisterUserResponse),
}
