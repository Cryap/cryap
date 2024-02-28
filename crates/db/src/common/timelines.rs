use chrono::{DateTime, Utc};
use diesel::{
    dsl::{not, sql},
    prelude::*,
    result::Error::NotFound,
    sql_query,
    sql_types::{Bool, Bpchar, Integer, Nullable, Text, Timestamptz, Varchar},
};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{Post, PostBoost},
    paginate,
    pagination::Pagination,
    schema::{posts, sql_types::Visibility, user_followers},
    types::{DbId, DbVisibility},
};

pub enum TimelineEntry {
    Post(Post),
    Boost(PostBoost, Post),
}

impl From<TimelineResult> for TimelineEntry {
    fn from(value: TimelineResult) -> Self {
        if value.post_type == "boost" {
            Self::Boost(
                PostBoost {
                    id: value.boost_id.unwrap(),
                    ap_id: value.boost_ap_id.unwrap(),
                    post_id: value.id.clone(),
                    actor_id: value.boost_actor_id.unwrap(),
                    visibility: value.boost_visibility.unwrap(),
                    published: value.published,
                },
                Post {
                    id: value.id,
                    author: value.author,
                    ap_id: value.ap_id,
                    local_only: value.local_only,
                    content_warning: value.content_warning,
                    content: value.content,
                    sensitive: value.sensitive,
                    in_reply: value.in_reply,
                    published: value.post_published.unwrap(),
                    updated: value.updated,
                    url: value.url,
                    quote: value.quote,
                    visibility: value.visibility,
                },
            )
        } else {
            Self::Post(Post {
                id: value.id,
                author: value.author,
                ap_id: value.ap_id,
                local_only: value.local_only,
                content_warning: value.content_warning,
                content: value.content,
                sensitive: value.sensitive,
                in_reply: value.in_reply,
                published: value.published,
                updated: value.updated,
                url: value.url,
                quote: value.quote,
                visibility: value.visibility,
            })
        }
    }
}

impl From<Post> for TimelineEntry {
    fn from(value: Post) -> Self {
        Self::Post(value)
    }
}

#[derive(QueryableByName, Debug)]
struct TimelineResult {
    #[diesel(sql_type = Varchar)]
    post_type: String,
    #[diesel(sql_type = Bpchar)]
    id: DbId,
    #[diesel(sql_type = Bpchar)]
    author: DbId,
    #[diesel(sql_type = Varchar)]
    ap_id: String,
    #[diesel(sql_type = Bool)]
    local_only: bool,
    #[diesel(sql_type = Nullable<Text>)]
    content_warning: Option<String>,
    #[diesel(sql_type = Text)]
    content: String,
    #[diesel(sql_type = Bool)]
    sensitive: bool,
    #[diesel(sql_type = Nullable<Bpchar>)]
    in_reply: Option<DbId>,
    #[diesel(sql_type = Timestamptz)]
    published: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    updated: Option<DateTime<Utc>>,
    #[diesel(sql_type = Varchar)]
    url: String,
    #[diesel(sql_type = Nullable<Bpchar>)]
    quote: Option<DbId>,
    #[diesel(sql_type = Visibility)]
    visibility: DbVisibility,
    #[diesel(sql_type = Nullable<Bpchar>)]
    boost_id: Option<DbId>,
    #[diesel(sql_type = Nullable<Varchar>)]
    boost_ap_id: Option<String>,
    #[diesel(sql_type = Nullable<Bpchar>)]
    boost_actor_id: Option<DbId>,
    #[diesel(sql_type = Nullable<Visibility>)]
    boost_visibility: Option<DbVisibility>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    post_published: Option<DateTime<Utc>>,
}

pub async fn get_user_posts(
    user_id: &DbId,
    pagination: Pagination,
    actor_id: Option<&DbId>,
    exclude_boosts: bool,
    exclude_replies: bool,
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<Vec<TimelineEntry>> {
    let is_follower = follows(user_id, actor_id, db_pool).await?;
    if exclude_boosts {
        let mut query = posts::table
            .filter(posts::author.eq(user_id))
            .select(posts::all_columns)
            .into_boxed();

        if exclude_replies {
            query = query.filter(posts::in_reply.is_null());
        }

        if !is_follower {
            query = query.filter(not(posts::visibility.eq(DbVisibility::Private)));
        }

        query = paginate!(query, posts::id, pagination);

        Ok(query
            .load::<Post>(&mut db_pool.get().await?)
            .await?
            .into_iter()
            .map(TimelineEntry::from)
            .collect())
    } else {
        let query = format!(
            "
            SELECT * FROM (
                SELECT
                    'post' AS post_type,
                    posts.id as id,
                    author,
                    ap_id,
                    local_only,
                    content_warning,
                    content,
                    sensitive,
                    in_reply,
                    published,
                    updated,
                    url,
                    quote,
                    visibility,
                    NULL AS boost_id,
                    NULL AS boost_ap_id,
                    NULL AS boost_actor_id,
                    NULL AS boost_visibility,
                    NULL AS post_published 
                FROM posts
                {}
                WHERE author = $2{}{}
                UNION ALL
                SELECT
                    'boost' AS post_type,
                    posts.id AS id,
                    posts.author AS author,
                    posts.ap_id AS ap_id,
                    posts.local_only AS local_only,
                    posts.content_warning AS content_warning,
                    posts.content AS content,
                    posts.sensitive AS sensitive,
                    posts.in_reply AS in_reply,
                    post_boost.published AS published,
                    posts.updated AS updated,
                    posts.url AS url,
                    posts.quote AS quote,
                    posts.visibility AS visibility,
                    post_boost.id AS boost_id,
                    post_boost.ap_id AS boost_ap_id,
                    post_boost.actor_id AS boost_actor_id,
                    post_boost.visibility AS boost_visibility,
                    posts.published AS post_published
                FROM post_boost
                JOIN posts ON post_boost.post_id = posts.id
                WHERE post_boost.actor_id = $2{} AND post_boost.visibility != 'direct'
            ) results {}
            ",
            if actor_id.is_some() {
                "LEFT JOIN post_mention ON posts.id = post_mention.post_id AND post_mention.mentioned_user_id = $1"
            } else {
                ""
            },
            if actor_id.is_some() {
                format!(" AND (({}posts.visibility != 'direct') OR ((posts.visibility = 'private' OR posts.visibility = 'direct') AND post_mention.post_id IS NOT NULL))", if is_follower {
                    ""
                } else {
                    "posts.visibility != 'private' AND "
                }).leak()
            } else {
                ""
            },
            if exclude_replies {
                " AND in_reply IS NULL"
            } else {
                ""
            },
            if is_follower {
                ""
            } else {
                " AND post_boost.visibility != 'private'"
            },
            match pagination {
                Pagination::MaxId(_, _) => {
                    "WHERE id > $3 ORDER BY published DESC LIMIT $4;"
                },
                Pagination::MinId(_, _) => {
                    "WHERE id < $3 ORDER BY published DESC LIMIT $4;"
                },
                Pagination::None(_) => "ORDER BY published DESC LIMIT $4",
            }
        );

        Ok(sql_query(query)
            .bind::<Bpchar, _>(actor_id.map(|id| id.to_string()).unwrap_or_default())
            .bind::<Bpchar, _>(user_id)
            .bind::<Varchar, _>(match pagination {
                Pagination::MaxId(ref id, _) | Pagination::MinId(ref id, _) => &id,
                Pagination::None(_) => "",
            })
            .bind::<Integer, _>(match pagination {
                Pagination::MaxId(_, limit)
                | Pagination::MinId(_, limit)
                | Pagination::None(limit) => limit,
            })
            .load::<TimelineResult>(&mut db_pool.get().await?)
            .await?
            .into_iter()
            .map(TimelineEntry::from)
            .collect())
    }
}

async fn follows(
    user_id: &DbId,
    actor_id: Option<&DbId>,
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<bool> {
    if let Some(actor_id) = actor_id {
        let result = user_followers::table
            .select(sql::<Bool>("true"))
            .filter(user_followers::actor_id.eq(actor_id))
            .filter(user_followers::follower_id.eq(user_id))
            .first::<bool>(&mut db_pool.get().await?)
            .await;
        match result {
            Ok(_) => Ok(true),
            Err(NotFound) => Ok(false),
            Err(err) => Err(err.into()),
        }
    } else {
        Ok(false)
    }
}
