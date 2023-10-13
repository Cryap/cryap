use std::sync::Arc;

use activitypub_federation::{
    activity_queue::send_activity, config::Data, http_signatures::generate_actor_keypair,
};
use anyhow::anyhow;
use ap::{activities::update::Update, objects::user::ApUser};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use db::{models::user::User, schema::users, types::DbId};
use diesel::{insert_into, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use url::Url;
use web::AppState;

pub const USERNAME_RE: &str = r"[a-z0-9_]+([a-z0-9_.-]+[a-z0-9_]+)?";
pub const MENTION_RE: &str = r"@(?P<name>[\w.]+)(@(?P<domain>[a-zA-Z0-9._:-]+))?";

pub async fn register(
    name: String,
    password: String,
    bio: Option<String>,
    display_name: Option<String>,
    state: &Arc<AppState>,
) -> anyhow::Result<ApUser> {
    let mut conn = state.db_pool.get().await?;
    let ap_id = format!("https://{}/u/{}", state.config.web.domain, name);

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
        shared_inbox_uri: None, //Some(format!("https://{}/inbox", state.config.web.domain)),
        outbox_uri: format!("{ap_id}/ap/outbox"),
        followers_uri: format!("{ap_id}/ap/followers"),
        name,
        instance: state.config.web.domain.clone(),
        display_name,
        bio,
        password_encrypted: Some(password_hash),
        admin: false,
        public_key: keypair.public_key,
        private_key: Some(keypair.private_key),
        published: Utc::now().naive_utc(),
        updated: Some(Utc::now().naive_utc()),
        manually_approves_followers: false,
        is_cat: false,
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

pub async fn get_instances(state: &Arc<AppState>) -> anyhow::Result<Vec<String>> {
    Ok(users::table
        .filter(users::local.eq(false))
        .distinct()
        .select(users::instance)
        .load(&mut state.db_pool.get().await?)
        .await?)
}

pub async fn distribute_update(user: &User, data: &Data<Arc<AppState>>) -> anyhow::Result<()> {
    send_activity(
        Update::build(user.clone(), data).await?,
        &ApUser(user.clone()),
        user.reached_inboxes(&data.db_pool)
            .await?
            .into_iter()
            .map(|inbox| Url::parse(&inbox))
            .collect::<Result<Vec<Url>, url::ParseError>>()?,
        data,
    )
    .await?;
    Ok(())
}
