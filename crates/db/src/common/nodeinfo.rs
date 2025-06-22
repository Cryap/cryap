use diesel::{dsl::sql, prelude::*, select, sql_types::Bool};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::schema::{posts, users};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfoUsage {
    pub users: Option<NodeInfoUsers>,
    pub local_posts: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfoUsers {
    pub total: Option<i64>,
    pub active_halfyear: Option<i64>,
    pub active_month: Option<i64>,
}

pub async fn get_nodeinfo_usage(
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<NodeInfoUsage> {
    let (users_count, posts_count): (Option<i64>, Option<i64>) = select((
        users::table
            .filter(users::local.eq(true))
            .count()
            .single_value(),
        posts::table
            .select(sql::<Bool>("true"))
            .inner_join(users::table.on(users::id.eq(posts::author)))
            .filter(users::local.eq(true))
            .count()
            .single_value(),
    ))
    .first(&mut db_pool.get().await?)
    .await?;

    Ok(NodeInfoUsage {
        users: Some(NodeInfoUsers {
            total: Some(users_count.unwrap_or(0)),
            active_halfyear: Some(0), // TODO
            active_month: Some(0),    // TODO
        }),
        local_posts: Some(posts_count.unwrap_or(0)),
    })
}
