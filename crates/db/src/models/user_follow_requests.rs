use diesel::prelude::*;

use crate::schema::user_follow_requests;
use crate::types::DbId;

#[derive(Queryable, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = user_follow_requests)]
pub struct UserFollowRequest {
    pub actor_id: DbId,
    pub follower_id: DbId,
    pub ap_id: Option<String>,
    pub published: chrono::NaiveDateTime,
}

#[derive(Clone, Insertable, AsChangeset)]
#[diesel(table_name = user_follow_requests)]
pub struct UserFollowRequestsInsert {
    pub actor_id: DbId,
    pub follower_id: DbId,
    pub ap_id: Option<String>,
}
