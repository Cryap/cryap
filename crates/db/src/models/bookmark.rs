use diesel::prelude::*;

use crate::{schema::bookmarks, types::DbId};

#[derive(Queryable, Insertable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = bookmarks)]
pub struct Bookmark {
    pub id: DbId,
    pub post_id: DbId,
    pub actor_id: DbId,
    pub published: chrono::NaiveDateTime,
}
