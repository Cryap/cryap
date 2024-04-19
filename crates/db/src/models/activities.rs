use chrono::{DateTime, Utc};
use diesel::{
    insert_into,
    prelude::*,
    result::{DatabaseErrorKind, Error::DatabaseError},
};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::schema::received_activities;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(primary_key(ap_id))]
#[diesel(table_name = received_activities)]
pub struct ReceivedActivity {
    pub ap_id: String,
    pub published: DateTime<Utc>,
}

impl ReceivedActivity {
    pub async fn create(
        ap_id: &str,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> Result<(), diesel::result::Error> {
        let rows_affected = insert_into(received_activities::table)
            .values(received_activities::ap_id.eq(ap_id))
            .on_conflict_do_nothing()
            .execute(&mut db_pool.get().await.unwrap())
            .await
            .optional()?;
        if rows_affected == Some(1) {
            // New activity inserted successfully
            Ok(())
        } else {
            // Duplicate activity
            Err(DatabaseError(
                DatabaseErrorKind::UniqueViolation,
                Box::<String>::default(),
            ))
        }
    }
}
