use std::sync::Arc;

use chrono::{DateTime, Utc};
use db::{
    models::{Notification as DbNotification, Post, User},
    types::{DbId, DbNotificationType},
};
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
        let account =
            Account::build(notification.actor(&state.db_pool).await?, state, false).await?;
        let status = match notification.post_id {
            Some(_) => Some(
                Status::build(
                    notification.post(&state.db_pool).await?.unwrap(), // Panic safety: post existence is checked before
                    Some(&notification.receiver_id),
                    state,
                )
                .await?,
            ),
            None => None,
        };

        Ok(Self::raw_build(notification, account, status))
    }

    pub async fn build_from_vec(
        notifications: Vec<DbNotification>,
        receiver_id: &DbId,
        state: &Arc<AppState>,
    ) -> anyhow::Result<Vec<Self>> {
        let accounts = Account::build_from_vec(
            User::by_ids(
                notifications
                    .iter()
                    .map(|notification| &notification.actor_id)
                    .collect(),
                &state.db_pool
            )
            .await?
            .into_iter()
            .map(|user| user.expect("complete deletion of a user is not possible; its presence is checked before creating a notification"))
            .collect::<Vec<User>>(),
            state,
        )
        .await?;

        let post_ids: Vec<Option<&DbId>> = notifications
            .iter()
            .map(|notification| notification.post_id.as_ref())
            .collect();
        let filtered_statuses = Status::build_from_vec(
            Post::by_ids(
                post_ids.iter().filter_map(|id| *id).collect(),
                &state.db_pool,
            )
            .await?
            .into_iter()
            .map(|post| post.expect("complete deletion of a post is not possible; its presence is checked before creating a notification"))
            .collect::<Vec<Post>>(),
            Some(receiver_id),
            state,
        )
        .await?;

        let mut filtered_statuses_iter = filtered_statuses.into_iter();
        let statuses: Vec<Option<Status>> = post_ids
            .into_iter()
            .map(|id| id.and_then(|_| filtered_statuses_iter.next()))
            .collect();

        Ok(notifications
            .into_iter()
            .zip(accounts)
            .zip(statuses)
            .map(|((notification, account), status)| Self::raw_build(notification, account, status))
            .collect())
    }

    fn raw_build(notification: DbNotification, account: Account, status: Option<Status>) -> Self {
        Self {
            id: notification.id.to_string(),
            account,
            status,
            notification_type: notification.notification_type,
            created_at: notification.published,
        }
    }
}
