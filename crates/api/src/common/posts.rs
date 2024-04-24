use std::sync::Arc;

use activitypub_federation::{
    config::Data, fetch::webfinger::webfinger_resolve_actor, traits::Actor,
};
use ap::{
    activities::{create::note::CreateNote, like::Like, undo::like::UndoLike},
    common::notifications,
    objects::{
        announce::{Announce, ApAnnounce},
        note::ApNote,
        user::ApUser,
    },
};
use chrono::Utc;
use db::{
    models::{Post, PostBoost, PostLike, PostMention, User},
    types::{DbId, DbVisibility},
};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use url::Url;
use web::AppState;

use super::users::MENTION_RE;

pub async fn accessible_for(
    post: &Post,
    user: Option<&User>,
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<bool> {
    if let Some(user) = user {
        match post.visibility {
            DbVisibility::Public | DbVisibility::Unlisted => Ok(true),
            DbVisibility::Private if post.is_mentioned(user, db_pool).await? => Ok(true),
            DbVisibility::Private => Ok(user.follows_by_id(&post.author, db_pool).await?),
            DbVisibility::Direct => Ok(post.is_mentioned(user, db_pool).await?),
        }
    } else {
        match post.visibility {
            DbVisibility::Public | DbVisibility::Unlisted => Ok(true),
            _ => Ok(false),
        }
    }
}

pub async fn boost_accessible_for(
    boost: &PostBoost,
    user: Option<&User>,
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<bool> {
    if let Some(user) = user {
        match boost.visibility {
            DbVisibility::Public | DbVisibility::Unlisted => Ok(true),
            DbVisibility::Private => Ok(user.follows_by_id(&boost.actor_id, db_pool).await?),
            DbVisibility::Direct => Ok(false),
        }
    } else {
        match boost.visibility {
            DbVisibility::Public | DbVisibility::Unlisted => Ok(true),
            _ => Ok(false),
        }
    }
}

pub async fn post_or_boost_by_id(
    id: &DbId,
    db_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<(Option<Post>, Option<PostBoost>)> {
    let post = Post::by_id(id, db_pool).await?;
    if let Some(post) = post {
        return Ok((Some(post), None));
    }

    let boost = PostBoost::by_id(id, db_pool).await?;
    if let Some(boost) = boost {
        return Ok((Some(boost.post(db_pool).await?), Some(boost)));
    }

    Ok((None, None))
}

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
    regex::Regex::new(MENTION_RE)
        .unwrap()
        .captures_iter(&content)
        .filter_map(|m| {
            m.get(0)
                .map(|m| m.as_str().to_string().trim()[1..].to_string()) // strip @ symbol
        })
        .collect()
}

pub async fn post(
    user: &User,
    options: NewPost,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<Post> {
    let id = DbId::default();
    let ap_id = format!(
        "https://{}/p/{}",
        data.config.web.domain,
        id.clone().to_string()
    );

    let mut content = options.content;
    let mut mentions: Vec<ApUser> = vec![];

    for mention in match_mentions(content.clone()) {
        let user = if mention.contains('@') {
            match User::by_acct(mention.clone(), &data.db_pool).await? {
                Some(user) => ApUser(user),
                None => match webfinger_resolve_actor(&mention, data).await {
                    Ok(user) => user,
                    Err(_) => continue,
                },
            }
        } else {
            match User::local_by_name(&mention, &data.db_pool).await? {
                Some(user) => ApUser(user),
                None => continue,
            }
        };

        content = content.replace(&format!("@{}", mention), &format!("<a class=\"u-url mention\" href=\"{}\" rel=\"ugc\" data-user=\"{}\">@<span>{}</span>@<span>{}</span></a>", user.ap_id, user.id.to_string(), user.name, user.instance));
        mentions.push(user);
    }

    let mentions_data: Vec<PostMention> = mentions
        .iter()
        .map(|mention| PostMention {
            id: DbId::default(),
            post_id: id.clone(),
            mentioned_user_id: mention.id.clone(),
        })
        .collect();

    let post = Post {
        id,
        url: ap_id.clone(),
        ap_id,
        updated: None,
        quote: options.quote.map(|p| p.id),
        author: user.id.clone(),
        content, // validation should be performed before post() call
        in_reply: options.in_reply.map(|p| p.id),
        sensitive: options.sensitive,
        published: Utc::now(),
        local_only: options.local_only,
        visibility: options.visibility,
        content_warning: options.content_warning,
    };

    if !options.local_only {
        let inboxes = if post.visibility == DbVisibility::Direct {
            mentions
                .iter()
                .filter(|mention| !mention.local)
                .map(|mention| mention.shared_inbox_or_inbox().to_string())
                .collect()
        } else {
            let mut inboxes = user.reached_inboxes(&data.db_pool).await?;
            inboxes.extend(
                mentions
                    .iter()
                    .map(|mention| mention.shared_inbox_or_inbox().to_string()),
            );
            inboxes
        };

        CreateNote::send(
            ApNote(post.clone()),
            &ApUser(user.clone()),
            &mentions,
            inboxes
                .into_iter()
                .map(|inbox| Url::parse(&inbox))
                .collect::<Result<Vec<Url>, url::ParseError>>()?,
            data,
        )
        .await?;
    }

    let post = Post::create(post, mentions_data, &data.db_pool).await?;
    notifications::process_post(&post, &data.db_pool).await?;
    Ok(post)
}

pub async fn boost(
    user: &User,
    post: &Post,
    visibility: DbVisibility,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<PostBoost> {
    let id = DbId::default();
    let ap_id = format!("{}/activities/boost/{}", user.ap_id, id.to_string());

    let boost = PostBoost {
        id,
        ap_id,
        post_id: post.id.clone(),
        actor_id: user.id.clone(),
        visibility,
        published: Utc::now(),
    };

    let author = post.author(&data.db_pool).await?;
    if !post.local_only {
        let user = ApUser(user.clone());
        let mut inboxes = user.reached_inboxes(&data.db_pool).await?;
        if !author.local {
            let author_inbox = user.shared_inbox_or_inbox().to_string();
            if !inboxes.contains(&author_inbox) {
                inboxes.push(author_inbox);
            }
        }

        Announce::send(
            ApAnnounce(boost.clone()),
            &user,
            inboxes
                .into_iter()
                .map(|inbox| Url::parse(&inbox))
                .collect::<Result<Vec<Url>, url::ParseError>>()?,
            data,
        )
        .await?;
    }

    let boost = PostBoost::create(boost, &data.db_pool).await?;
    notifications::process_boost(post, user, &author, false, &data.db_pool).await?;
    Ok(boost)
}

pub async fn like(user: &User, post: &Post, data: &Data<Arc<AppState>>) -> anyhow::Result<()> {
    let author = post.author(&data.db_pool).await?;
    let id = if author.local {
        None
    } else {
        Some(
            Like::send(
                &ApUser(user.clone()),
                &ApUser(author.clone()),
                &ApNote(post.clone()),
                data,
            )
            .await?
            .to_string(),
        )
    };

    PostLike::create(id, &post, &user, &data.db_pool).await?;
    notifications::process_like(post, user, &author, false, &data.db_pool).await?;

    Ok(())
}

pub async fn unlike(user: &User, post: &Post, data: &Data<Arc<AppState>>) -> anyhow::Result<()> {
    let author = post.author(&data.db_pool).await?;
    if !author.local {
        let like = PostLike::by_post_and_actor(post, user, &data.db_pool).await?;
        if let Some(like) = like {
            let like_id = like.ap_id.unwrap(); // Panic safety: only local likes don't have ap_id
            UndoLike::send(
                Url::parse(&like_id)?,
                &ApUser(user.clone()),
                &ApUser(author.clone()),
                &ApNote(post.clone()),
                data,
            )
            .await?;
        } else {
            return Ok(());
        }
    }

    PostLike::delete(None, &post, &user, &data.db_pool).await?;
    notifications::process_like(post, user, &author, true, &data.db_pool).await?;

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
