use diesel::prelude::*;

use crate::{schema::user_followers, types::DbId};

#[derive(Queryable, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = user_followers)]
pub struct UserFollower {
    pub actor_id: DbId,
    pub follower_id: DbId,
    pub ap_id: Option<String>,
    pub published: chrono::NaiveDateTime,
}

#[derive(Clone, Insertable, AsChangeset)]
#[diesel(table_name = user_followers)]
pub struct UserFollowersInsert {
    pub actor_id: DbId,
    pub follower_id: DbId,
    pub ap_id: Option<String>,
}
