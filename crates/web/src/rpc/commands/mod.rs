pub mod register;
pub mod userfetch;

use serde::{Deserialize, Serialize};

use crate::rpc::commands::register::{RpcRegisterUserData, RpcRegisterUserResponse};
use crate::rpc::commands::userfetch::RpcUserFetchResponse;

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "content")]
pub enum RpcCommandData {
    UserFetch(String),
    RegisterUser(RpcRegisterUserData),
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "content")]
pub enum RpcCommandResponse {
    UserFetch(RpcUserFetchResponse),
    RegisterUser(RpcRegisterUserResponse),
}
