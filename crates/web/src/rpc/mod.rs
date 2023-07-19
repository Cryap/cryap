mod commands;

use std::fs;
use std::sync::Arc;

use activitypub_federation::config::Data;
use tokio::io;
use tokio::net::{UnixListener, UnixStream};

use crate::rpc::commands::register::RpcRegisterUser;
use crate::rpc::commands::userfetch::RpcUserFetch;
use crate::rpc::commands::{RpcCommandData, RpcCommandResponse};
use crate::AppState;

pub async fn process(stream: UnixStream, data: Arc<Data<Arc<AppState>>>) -> anyhow::Result<()> {
    let mut msg = vec![0; 1024];

    loop {
        // Wait for the socket to be readable
        stream.readable().await?;

        // Try to read request, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        let request: String = match stream.try_read(&mut msg) {
            Ok(n) if n > 0 => String::from_utf8_lossy(&msg[0..n]).to_string(),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
            _ => {
                continue;
            }
        };

        let request = serde_json::from_str::<RpcCommandData>(&request);

        if let Ok(request) = request {
            let response = match request {
                RpcCommandData::UserFetch(request) => {
                    RpcCommandResponse::UserFetch(RpcUserFetch::call(request, &data).await)
                }
                RpcCommandData::RegisterUser(request) => {
                    RpcCommandResponse::RegisterUser(RpcRegisterUser::call(request, &data).await)
                }
            };

            loop {
                // Wait for the socket to be writable
                stream.writable().await?;

                let mut response = serde_json::to_string(&response).unwrap();
                response.push('\n');

                // Try to write data, this may still fail with `WouldBlock`
                // if the readiness event is a false positive.
                match stream.try_write(response.as_bytes()) {
                    Ok(_) => {
                        break;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
        } else {
            log::error!("Invalid RPC command {:#?}", request);
        }
    }
}

pub async fn start(data: Arc<Data<Arc<AppState>>>) {
    let _ = fs::remove_file("cryap.rpc");
    let listener = UnixListener::bind("cryap.rpc").unwrap();
    loop {
        if let Ok((stream, _addr)) = listener.accept().await {
            tokio::spawn(process(stream, Arc::clone(&data)));
        }
    }
}
