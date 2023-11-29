use std::sync::Arc;

use chrono::{DateTime, Utc};
use db::{models::Notification as DbNotification, types::DbNotificationType};
use futures::future::join_all;
use serde::Serialize;
use serde_with::skip_serializing_none;
use web::AppState;

use crate::entities::{Account, Status};

#[skip_serializing_none]
#[derive(Clone, Serialize)]
pub struct Notification {
    pub id: String,
    pub account: Account,
    pub status: Option<Status>,
    #[serde(rename = "type")]
    pub notification_type: DbNotificationType,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    pub async fn build(
        notification: DbNotification,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            id: notification.id.to_string(),
            account: Account::build(notification.actor(&state.db_pool).await?, state, false)
                .await?,
            status: match notification.post_id {
                Some(_) => Some(
                    Status::build(
                        notification.post(&state.db_pool).await?.unwrap(), // Panic safety: post existence is checked before
                        Some(&notification.actor(&state.db_pool).await?),
                        state,
                    )
                    .await?,
                ),
                None => None,
            },
            notification_type: notification.notification_type,
            created_at: notification.published,
        })
    }

    pub async fn build_from_vec(
        notifications: Vec<DbNotification>,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Vec<Self>> {
        join_all(
            notifications
                .into_iter()
                .map(|notification| async { Self::build(notification, state).await }),
        )
        .await
        .into_iter()
        .collect()
    }
}
