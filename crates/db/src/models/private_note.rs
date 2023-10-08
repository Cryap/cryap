use diesel::{delete, insert_into, prelude::*, result::Error::NotFound};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};

use crate::{models::User, schema::private_notes, types::DbId};

#[derive(Queryable, Insertable, AsChangeset, Selectable, Debug, PartialEq, Clone, Eq)]
#[diesel(table_name = private_notes)]
pub struct PrivateNote {
    pub id: DbId,
    pub actor_id: DbId,
    pub user_id: DbId,
    pub note: String,
}

impl PrivateNote {
    pub async fn get(
        actor: &User,
        user: &User,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<Option<String>> {
        let note = private_notes::table
            .filter(private_notes::actor_id.eq(&actor.id))
            .filter(private_notes::user_id.eq(&user.id))
            .select(private_notes::note)
            .first::<String>(&mut db_pool.get().await?)
            .await;
        match note {
            Ok(note) => Ok(Some(note)),
            Err(NotFound) => Ok(Some(String::new())),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn set(
        actor: &User,
        user: &User,
        note: Option<&String>,
        db_pool: &Pool<AsyncPgConnection>,
    ) -> anyhow::Result<()> {
        if let Some(note) = note {
            let private_note = PrivateNote {
                id: DbId::default(),
                actor_id: actor.id.clone(),
                user_id: user.id.clone(),
                note: note.clone(),
            };

            insert_into(private_notes::table)
                .values(private_note.clone())
                .on_conflict((private_notes::actor_id, private_notes::user_id))
                .do_update()
                .set(private_note)
                .execute(&mut db_pool.get().await?)
                .await?;
        } else {
            let _ = delete(
                private_notes::table
                    .filter(private_notes::actor_id.eq(&actor.id))
                    .filter(private_notes::user_id.eq(&user.id)),
            )
            .execute(&mut db_pool.get().await?)
            .await;
        }

        Ok(())
    }
}
