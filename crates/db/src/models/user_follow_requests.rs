use diesel::prelude::*;

use crate::schema::user_follow_requests;
use crate::types::DbId;

#[derive(Clone, Insertable, AsChangeset)]
#[diesel(table_name = user_follow_requests)]
pub struct UserFollowRequestsInsert {
    pub actor_id: DbId,
    pub follower_id: DbId,
}
