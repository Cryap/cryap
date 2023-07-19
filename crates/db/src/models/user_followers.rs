use diesel::prelude::*;

use crate::schema::user_followers;
use crate::types::DbId;

#[derive(Clone, Insertable, AsChangeset)]
#[diesel(table_name = user_followers)]
pub struct UserFollowersInsert {
    pub actor_id: DbId,
    pub follower_id: DbId,
}
