use std::{collections::HashSet, sync::Arc};

use ap::common::streaming::{StreamingCategory, StreamingEvent, EVENT_BUS};
use async_stream::try_stream;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    handler::Handler,
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::get,
    Extension, Router,
};
use db::models::Session;
use futures::{stream::Stream, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use web::{errors::AppError, AppState};

use crate::{auth_middleware::auth_middleware, entities::Notification, error::ApiError};

// https://docs.joinmastodon.org/methods/streaming/#health
pub async fn http_get_health() -> impl IntoResponse {
    String::from("OK").into_response()
}

// https://docs.joinmastodon.org/methods/streaming/#notification
pub async fn http_get_user_notification(
    state: State<Arc<AppState>>,
    Extension(session): Extension<Session>,
) -> Sse<impl Stream<Item = anyhow::Result<Event>>> {
    let mut stream = EVENT_BUS.get_receiver(&session.user_id).await;
    Sse::new(try_stream! {
        yield Event::default().comment(")");
        while let Ok(event) = stream.recv().await {
            match event {
                StreamingEvent::Notification { payload, categories } if categories.contains(&StreamingCategory::UserNotification) => {
                    yield Event::default().event("notification").json_data(Notification::build(payload, &state).await?)?;
                },
                _ => {}
            }
        }
    })
    .keep_alive(KeepAlive::default().text("thump"))
}

#[derive(Deserialize)]
pub struct WebSocketQuery {
    access_token: Option<String>,
    stream: Option<String>,
}

// https://docs.joinmastodon.org/methods/streaming/#websocket
pub async fn http_get_websocket(
    ws: WebSocketUpgrade,
    state: State<Arc<AppState>>,
    Query(query): Query<WebSocketQuery>,
) -> Result<impl IntoResponse, AppError> {
    let session = match query.access_token {
        Some(token) => match Session::by_token(&token, &state.db_pool).await? {
            Some(session) => session,
            None => {
                return Ok(StatusCode::UNAUTHORIZED.into_response());
            },
        },
        None => {
            return Ok(StatusCode::UNAUTHORIZED.into_response());
        },
    };

    Ok(ws.on_upgrade(move |socket| handle_websocket(socket, session, query.stream, state)))
}

#[derive(Serialize)]
struct WebSocketEvent {
    stream: Vec<String>,
    event: String,
    payload: String,
}

#[derive(Deserialize)]
struct WebSocketMessage {
    #[serde(rename = "type")]
    message_type: String,
    stream: String,
}

async fn handle_websocket(
    socket: WebSocket,
    session: Session,
    stream: Option<String>,
    state: State<Arc<AppState>>,
) {
    let mut event_receiver = EVENT_BUS.get_receiver(&session.user_id).await;
    let (mut split_sink, mut split_stream) = socket.split();

    let socket_categories = Arc::new(Mutex::new(match stream {
        Some(stream) => match StreamingCategory::by_name(&stream) {
            Some(category) => HashSet::from([category]),
            None => {
                if split_sink
                    .send(Message::Text(
                        serde_json::to_string(&ApiError::new_without_status_code(
                            "Unknown stream type",
                        ))
                        .unwrap(), // Panic safety: hardcoded object
                    ))
                    .await
                    .is_err()
                {
                    return;
                }

                HashSet::new()
            },
        },
        None => HashSet::new(),
    }));

    let (sender, mut receiver) = mpsc::channel::<Message>(16);
    let mut send_task = tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if split_sink.send(message).await.is_err() {
                return;
            }
        }
    });

    let stream_task_sender = sender.clone();
    let stream_task_socket_categories = socket_categories.clone();
    let mut stream_task = tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            let socket_categories = stream_task_socket_categories.lock().await;
            match event {
                StreamingEvent::Notification {
                    payload,
                    categories,
                } if categories
                    .iter()
                    .any(|category| socket_categories.contains(category)) =>
                {
                    if stream_task_sender
                        .send(Message::Text(
                            serde_json::to_string(&WebSocketEvent {
                                stream: categories
                                    .into_iter()
                                    .filter(|category| socket_categories.contains(category))
                                    .map(|category| category.name())
                                    .collect(),
                                event: String::from("notification"),
                                payload: serde_json::to_string(
                                    match &Notification::build(payload, &state).await {
                                        Ok(notification) => notification,
                                        Err(error) => {
                                            log::error!("Error from route, {:#?}", error);
                                            return;
                                        },
                                    },
                                )
                                .unwrap(), // Panic safety: I hope it doesn't break
                            })
                            .unwrap(), // Panic safety: I hope it doesn't break
                        ))
                        .await
                        .is_err()
                    {
                        return;
                    }
                },
                _ => {},
            };
        }
    });

    let receive_task_sender = sender.clone();
    let receive_task_socket_categories = socket_categories.clone();
    let mut receive_task = tokio::spawn(async move {
        while let Some(Ok(message)) = split_stream.next().await {
            match message {
                Message::Text(text) => {
                    if let Ok(payload) = serde_json::from_str::<WebSocketMessage>(&text) {
                        if payload.message_type == "subscribe"
                            || payload.message_type == "unsubscribe"
                        {
                            if let Some(category) = StreamingCategory::by_name(&payload.stream) {
                                let mut socket_categories =
                                    receive_task_socket_categories.lock().await;
                                match payload.message_type.as_str() {
                                    "subscribe" => (*socket_categories).insert(category),
                                    "unsubscribe" => (*socket_categories).remove(&category),
                                    _ => false, // Crutch: insert and remove return bool type so this is necessary for this line to meet the requirement.
                                                // Match return value is then ignored
                                };
                            } else if receive_task_sender
                                .send(Message::Text(
                                    serde_json::to_string(&ApiError::new_without_status_code(
                                        "Unknown stream type",
                                    ))
                                    .unwrap(), // Panic safety: hardcoded object
                                ))
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                },
                Message::Close(_) => return,
                _ => {},
            };
        }
    });

    // If one of the tasks exit, abort the other
    tokio::select! {
        _ = (&mut send_task) => {
            stream_task.abort();
            receive_task.abort();
        },
        _ = (&mut stream_task) => {
            send_task.abort();
            receive_task.abort();
        },
        _ = (&mut receive_task) => {
            send_task.abort();
            stream_task.abort();
        }
    }
}

pub fn streaming(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/streaming/health", get(http_get_health))
        .route(
            "/api/v1/streaming/user/notification",
            get(http_get_user_notification
                .layer(from_fn_with_state(Arc::clone(state), auth_middleware))),
        )
        .route("/api/v1/streaming", get(http_get_websocket))
}
