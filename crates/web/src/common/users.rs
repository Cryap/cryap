use std::sync::Arc;

use activitypub_federation::http_signatures::generate_actor_keypair;
use anyhow::anyhow;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use db::{models::user::User, schema::users, types::DbId};
use diesel::insert_into;
use diesel_async::RunQueryDsl;

use crate::{ApUser, AppState};

pub async fn register(
    name: String,
    password: String,
    bio: Option<String>,
    display_name: Option<String>,
    data: &Arc<AppState>,
) -> Result<ApUser, anyhow::Error> {
    let mut conn = data.db_pool.get().await?;
    let ap_id = format!("https://{}/u/{}", std::env::var("CRYAP_DOMAIN")?, name);

    let keypair = generate_actor_keypair()?;

    let password_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|f| f.to_string())
    })
    .await?;

    let password_hash = match password_hash {
        Ok(hash) => hash,
        Err(_) => return Err(anyhow!("password hashing failed")),
    };

    let user = User {
        id: DbId::default(),
        ap_id: ap_id.clone(),
        local: true,
        inbox_uri: format!("{ap_id}/ap/inbox"),
        shared_inbox_uri: None, //Some(format!("https://{}/inbox", std::env::var("CRYAP_DOMAIN")?)),
        outbox_uri: format!("{ap_id}/ap/outbox"),
        followers_uri: format!("{ap_id}/ap/followers"),
        name,
        instance: std::env::var("CRYAP_DOMAIN")?,
        display_name,
        bio,
        password_encrypted: Some(password_hash), // TODO: Hash password
        admin: false,
        public_key: keypair.public_key,
        private_key: Some(keypair.private_key),
        published: Utc::now().naive_utc(),
        updated: Some(Utc::now().naive_utc()),
        manually_approves_followers: false,
    };

    Ok(ApUser(
        insert_into(users::table)
            .values(user.clone())
            .on_conflict(users::ap_id)
            .do_update()
            .set(user)
            .get_result::<User>(&mut conn)
            .await?,
    ))
}
