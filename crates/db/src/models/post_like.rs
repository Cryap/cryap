use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::{schema::post_like, types::DbId};

#[derive(Queryable, Insertable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = post_like)]
pub struct PostLike {
    pub ap_id: String,
    pub post_id: DbId,
    pub actor_id: DbId,
    pub published: DateTime<Utc>,
}
