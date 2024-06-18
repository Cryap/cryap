use anyhow::anyhow;
use chrono::{DateTime, Utc};
use diesel::{
    delete,
    dsl::{exists, sql},
    insert_into,
    pg::sql_types::Array,
    prelude::*,
    result::Error::NotFound,
    select, sql_query,
    sql_types::{BigInt, Bool, Bpchar},
};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{Bookmark, PostBoost, User},
    paginate,
    pagination::Pagination,
    schema::{bookmarks, post_boost, post_like, post_mention, posts, users},
    types::{DbId, DbVisibility},
};

#[derive(
    Queryable, Insertable, Identifiable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq,
)]
#[diesel(table_name = posts)]
pub struct Post {
    pub id: DbId,
    pub author: DbId,
    pub ap_id: String,
    pub local_only: bool,
    pub content_warning: Option<String>,
    pub content: String,
    pub sensitive: bool,
    pub in_reply: Option<DbId>,
    pub published: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub url: String,
    pub quote: Option<DbId>,
    pub visibility: DbVisibility,
}

pub struct PostRelationship {
    pub post_id: DbId,
    pub liked: bool,
    pub boosted: bool,
    pub bookmarked: bool,
}

#[derive(QueryableByName, Debug)]
pub struct PostStats {
    #[diesel(sql_type = Bpchar)]
    pub post_id: DbId,
    #[diesel(sql_type = BigInt)]
    pub likes_count: i64,
    #[diesel(sql_type = BigInt)]
    pub boosts_count: i64,
}

impl Post {
    pub async fn create(
        post: Self,
        mentions: Vec<PostMention>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Self> {
        let mut conn = db_pool.get().await?;

        let post = insert_into(posts::table)
            .values(vec![post])
            .get_result::<Self>(&mut conn)
            .await?;

        insert_into(post_mention::table)
            .values(mentions)
            .on_conflict((post_mention::post_id, post_mention::mentioned_user_id))
            .do_nothing()
            .execute(&mut conn)
            .await?;

        Ok(post)
    }

    pub async fn by_id(
        id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<Self>> {
        let post = posts::table
            .filter(posts::id.eq(id))
            .first::<Self>(&mut db_pool.get().await?)
            .await;
        match post {
            Ok(post) => Ok(Some(post)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn by_ids(
        ids: Vec<&DbId>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<Option<Self>>> {
        let posts = posts::table
            .filter(posts::id.eq_any(&ids))
            .load::<Self>(&mut db_pool.get().await?)
            .await?;
        Ok(ids
            .into_iter()
            .map(|id| posts.iter().find(|post| post.id == *id).cloned())
            .collect())
    }

    pub async fn relationships(
        ids: Vec<&DbId>,
        user_id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<PostRelationship>> {
        let tuples = posts::table
            .select((
                posts::id,
                exists(
                    post_like::table
                        .select(sql::<Bool>("true"))
                        .filter(post_like::post_id.eq(posts::id))
                        .filter(post_like::actor_id.eq(user_id)),
                ),
                exists(
                    post_boost::table
                        .select(sql::<Bool>("true"))
                        .filter(post_boost::post_id.eq(posts::id))
                        .filter(post_boost::actor_id.eq(user_id)),
                ),
                exists(
                    bookmarks::table
                        .select(sql::<Bool>("true"))
                        .filter(bookmarks::post_id.eq(posts::id))
                        .filter(bookmarks::actor_id.eq(user_id)),
                ),
            ))
            .filter(posts::id.eq_any(ids))
            .load::<(DbId, bool, bool, bool)>(&mut db_pool.get().await?)
            .await?;

        Ok(tuples
            .into_iter()
            .map(|(post_id, liked, boosted, bookmarked)| PostRelationship {
                post_id,
                liked,
                boosted,
                bookmarked,
            })
            .collect())
    }

    pub async fn stats(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<PostStats> {
        let (likes_count, boosts_count): (Option<i64>, Option<i64>) = select((
            post_like::table
                .filter(post_like::post_id.eq(&self.id))
                .count()
                .single_value(),
            post_boost::table
                .filter(post_boost::post_id.eq(&self.id))
                .count()
                .single_value(),
        ))
        .first(&mut db_pool.get().await?)
        .await?;

        Ok(PostStats {
            post_id: self.id.clone(),
            likes_count: likes_count.unwrap_or(0),
            boosts_count: boosts_count.unwrap_or(0),
        })
    }

    pub async fn stats_by_vec(
        ids: Vec<&DbId>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<PostStats>> {
        Ok(sql_query(
            "
            SELECT
                ids.id AS post_id,
                COUNT(post_like.*) AS likes_count,
                COUNT(post_boost.*) AS boosts_count
            FROM (SELECT unnest($1) AS id) AS ids
            LEFT JOIN post_like ON post_like.post_id = ids.id
            LEFT JOIN post_boost ON post_boost.post_id = ids.id 
            GROUP BY ids.id;
            ",
        )
        .bind::<Array<Bpchar>, _>(ids)
        .load::<PostStats>(&mut db_pool.get().await?)
        .await?)
    }

    pub async fn author(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<User> {
        User::by_id(&self.author, db_pool)
            .await?
            .ok_or(anyhow!("This wasn't supposed to happen"))
    }

    pub async fn liked_by(
        &self,
        pagination: Pagination,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<User>> {
        let query = post_like::table
            .filter(post_like::post_id.eq(&self.id))
            .inner_join(users::table.on(users::id.eq(post_like::actor_id)))
            .select(users::all_columns)
            .into_boxed();
        let query = paginate!(query, users::id, pagination);

        Ok(query.load::<User>(&mut db_pool.get().await?).await?)
    }

    pub async fn boosted_by(
        &self,
        pagination: Pagination,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<User>> {
        let query = post_boost::table
            .filter(post_boost::post_id.eq(&self.id))
            .inner_join(users::table.on(users::id.eq(post_boost::actor_id)))
            .select(users::all_columns)
            .into_boxed();
        let query = paginate!(query, users::id, pagination);

        Ok(query.load::<User>(&mut db_pool.get().await?).await?)
    }

    pub async fn is_liked_by(
        &self,
        user_id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let result = post_like::table
            .select(sql::<Bool>("true"))
            .filter(post_like::post_id.eq(&self.id))
            .filter(post_like::actor_id.eq(user_id))
            .first::<bool>(&mut db_pool.get().await?)
            .await;
        match result {
            Ok(_) => Ok(true),
            Err(NotFound) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn boost_by(
        &self,
        user_id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<PostBoost>> {
        let boost = post_boost::table
            .filter(post_boost::post_id.eq(&self.id))
            .filter(post_boost::actor_id.eq(user_id))
            .first::<PostBoost>(&mut db_pool.get().await?)
            .await;
        match boost {
            Ok(boost) => Ok(Some(boost)),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn is_mentioned(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<bool> {
        let result = post_mention::table
            .select(sql::<Bool>("true"))
            .filter(post_mention::post_id.eq(&self.id))
            .filter(post_mention::mentioned_user_id.eq(&user.id))
            .first::<bool>(&mut db_pool.get().await?)
            .await;
        match result {
            Ok(_) => Ok(true),
            Err(NotFound) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn relationship(
        &self,
        user_id: &DbId,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<PostRelationship> {
        let (liked, boosted, bookmarked): (Option<bool>, Option<bool>, Option<bool>) = select((
            post_like::table
                .select(sql::<Bool>("true"))
                .filter(post_like::post_id.eq(&self.id))
                .filter(post_like::actor_id.eq(user_id))
                .single_value(),
            post_boost::table
                .select(sql::<Bool>("true"))
                .filter(post_boost::post_id.eq(&self.id))
                .filter(post_boost::actor_id.eq(user_id))
                .single_value(),
            bookmarks::table
                .select(sql::<Bool>("true"))
                .filter(bookmarks::post_id.eq(&self.id))
                .filter(bookmarks::actor_id.eq(user_id))
                .single_value(),
        ))
        .first(&mut db_pool.get().await?)
        .await?;

        Ok(PostRelationship {
            post_id: self.id.clone(),
            liked: liked.unwrap_or_default(),
            boosted: boosted.unwrap_or_default(),
            bookmarked: bookmarked.unwrap_or_default(),
        })
    }

    pub async fn bookmark(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Bookmark> {
        let bookmark = Bookmark {
            id: DbId::default(),
            actor_id: user.id.clone(),
            post_id: self.id.clone(),
            published: Utc::now(),
        };

        insert_into(bookmarks::table)
            .values(vec![bookmark.clone()])
            .on_conflict((bookmarks::actor_id, bookmarks::post_id))
            .do_nothing()
            .execute(&mut db_pool.get().await?)
            .await?;

        Ok(bookmark)
    }

    pub async fn unbookmark(
        &self,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<()> {
        let _ = delete(
            bookmarks::table
                .filter(bookmarks::actor_id.eq(user.id.clone()))
                .filter(bookmarks::post_id.eq(self.id.clone())),
        )
        .execute(&mut db_pool.get().await?)
        .await;

        Ok(())
    }

    pub async fn mentioned_users(
        &self,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<User>> {
        Ok(post_mention::table
            .filter(post_mention::post_id.eq(&self.id))
            .inner_join(users::table)
            .select(User::as_select())
            .load(&mut db_pool.get().await?)
            .await?)
    }

    pub async fn local_mentioned_users(
        &self,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Vec<User>> {
        Ok(post_mention::table
            .filter(post_mention::post_id.eq(&self.id))
            .inner_join(users::table.on(users::id.eq(post_mention::mentioned_user_id)))
            .filter(users::local.eq(true))
            .select(users::all_columns)
            .load::<User>(&mut db_pool.get().await?)
            .await?)
    }
}

#[derive(Queryable, Insertable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = post_mention)]
pub struct PostMention {
    pub id: DbId,
    pub post_id: DbId,
    pub mentioned_user_id: DbId,
}
