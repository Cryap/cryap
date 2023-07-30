use std::collections::HashMap;

use anyhow::anyhow;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use redis::{aio::ConnectionManager, AsyncCommands};

use crate::{models::User, types::DbId, utils::random_string};

pub struct RedirectCode {
    pub code: String,
    pub client_id: String,
    pub user_id: DbId,
}

impl RedirectCode {
    pub async fn create(
        client_id: String,
        user_id: DbId,
        redis: &mut ConnectionManager,
    ) -> anyhow::Result<Self> {
        let code = random_string(32);
        let key = format!("codes:{}", code);

        redis
            .hset_multiple(
                &key,
                &[
                    ("code", &code),
                    ("client_id", &client_id),
                    ("user_id", &user_id.to_string()),
                ],
            )
            .await?;

        redis.expire(&key, 60).await?;

        Ok(RedirectCode {
            code,
            client_id,
            user_id,
        })
    }

    pub async fn by_code(
        code: &str,
        redis: &mut ConnectionManager,
    ) -> anyhow::Result<Option<Self>> {
        let hash: HashMap<String, String> = redis.hgetall(format!("codes:{}", code)).await?;

        if hash.is_empty() {
            Ok(None)
        } else {
            // Panic safety: should never panic if there was no external intervention in the Redis database
            Ok(Some(RedirectCode {
                code: hash.get("code").unwrap().clone(),
                client_id: hash.get("client_id").unwrap().clone(),
                user_id: hash.get("user_id").unwrap().clone().into(),
            }))
        }
    }

    pub async fn user(&self, db_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<User> {
        match User::by_id(&self.user_id, db_pool).await? {
            Some(user) => Ok(user),
            None => Err(anyhow!("User sucked into a black hole")),
        }
    }

    pub async fn delete(&self, redis: &mut ConnectionManager) -> anyhow::Result<()> {
        redis.del(format!("codes:{}", self.code)).await?;
        Ok(())
    }
}
