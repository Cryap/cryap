use std::sync::Arc;

use activitypub_federation::{
    activity_queue::send_activity, config::Data, fetch::webfinger::webfinger_resolve_actor,
    traits::Object,
};
use anyhow::anyhow;
use ap::{
    activities::create::note::CreateNote,
    objects::{note::ApNote, user::ApUser},
};
use chrono::Utc;
use db::{
    models::{user::User, Post, PostMention},
    schema::{post_mention, posts},
    types::{DbId, DbVisibility},
};
use diesel::insert_into;
use diesel_async::RunQueryDsl;
use web::AppState;

use super::users::MENTION_RE;

pub struct NewPost {
    pub visibility: DbVisibility,
    pub content: String,
    pub in_reply: Option<Post>,
    pub quote: Option<Post>,
    pub local_only: bool,
    pub sensitive: bool,
    pub content_warning: Option<String>,
}

fn match_mentions(content: String) -> Vec<String> {
    let mut mentions = vec![];

    let re = regex::Regex::new(MENTION_RE).unwrap();

    for mention in re.captures_iter(&content).map(|m| {
        m.get(0)
            .map(|m| m.as_str().to_string().trim()[1..].to_string()) // strip @ symbol
    }) {
        if let Some(mention) = mention {
            mentions.push(mention);
        }
    }

    mentions
}

pub async fn post(
    by: &User,
    options: NewPost,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<()> {
    let mut conn = data.db_pool.get().await?;
    let id = DbId::default();
    let ap_id = format!(
        "https://{}/u/{}",
        std::env::var("CRYAP_DOMAIN")?,
        id.clone().to_string()
    );

    let content = options.content;

    let mut mentions: Vec<ApUser> = vec![];

    for mention in match_mentions(content.clone()) {
        mentions.push(if mention.contains("@") {
            webfinger_resolve_actor(&mention, data).await?
        } else {
            let user = User::by_name(&mention, &data.db_pool).await?;
            match user {
                Some(user) => ApUser(user),
                None => return Err(anyhow!("mentioned local user not found")),
            }
        });
    }

    let object = Post {
        id: id.clone(),
        url: ap_id.clone(),
        ap_id,
        updated: None,
        quote: options.quote.map(|p| p.id),
        author: by.id.clone(),
        content, // validation should be performed before post() call
        in_reply: options.in_reply.map(|p| p.id),
        sensitive: options.sensitive,
        published: Utc::now().naive_utc(),
        local_only: options.local_only,
        visibility: options.visibility,
        content_warning: options.content_warning,
    };

    let mentions: Vec<PostMention> = mentions
        .iter()
        .map(move |mention| PostMention {
            id: DbId::default(),
            post_id: id.clone(),
            mentioned_user_id: mention.id.clone(),
        })
        .collect();

    if !options.local_only {
        let activity = CreateNote::from(ApNote(object.clone()).into_json(data).await?);

        let inboxes = vec![];
        send_activity(activity, &ApUser(by.clone()), inboxes, &data).await?;
    }

    insert_into(posts::dsl::posts)
        .values(vec![object])
        .execute(&mut conn)
        .await?;

    insert_into(post_mention::dsl::post_mention)
        .values(mentions)
        .on_conflict((
            post_mention::dsl::post_id,
            post_mention::dsl::mentioned_user_id,
        ))
        .do_nothing()
        .execute(&mut conn)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::common::posts::match_mentions;

    #[test]
    fn mentions() {
        let result = match_mentions("Hi!".to_string());
        assert!(result.len() == 0);

        let result = match_mentions("@vector1dev Hi!".to_string());
        assert_eq!(result, vec!["vector1dev"]);

        let result = match_mentions("@vector1dev @maksales@example.com Hi!".to_string());
        assert_eq!(result, vec!["vector1dev", "maksales@example.com"]);

        let result = match_mentions("@cryap&@vector1dev".to_string());
        assert_eq!(result, vec!["cryap", "vector1dev"]);
    }
}
