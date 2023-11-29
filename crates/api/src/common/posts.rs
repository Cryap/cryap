use std::sync::Arc;

use activitypub_federation::{
    activity_queue::send_activity,
    config::Data,
    fetch::{object_id::ObjectId, webfinger::webfinger_resolve_actor},
    traits::{Actor, Object},
};
use anyhow::anyhow;
use ap::{
    activities::{create::note::CreateNote, like::Like, undo::like::UndoLike},
    common::notifications,
    objects::{announce::ApAnnounce, note::ApNote, user::ApUser},
};
use chrono::Utc;
use db::{
    models::{Post, PostBoost, PostLike, PostMention, User},
    schema::{post_boost, post_like, post_mention, posts},
    types::{DbId, DbVisibility},
};
use diesel::{delete, insert_into, ExpressionMethods, QueryDsl};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
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

pub async fn post(by: &User, options: NewPost, data: &Data<Arc<AppState>>) -> anyhow::Result<Post> {
    let mut conn = data.db_pool.get().await?;
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
            webfinger_resolve_actor(&mention, data).await?
        } else {
            let user = User::by_name(&mention, &data.db_pool).await?;
            match user {
                Some(user) => ApUser(user),
                None => return Err(anyhow!("mentioned local user not found")),
            }
        };

        content = content.replace(&format!("@{}", mention), &format!("<a class=\"u-url mention\" href=\"{}\" rel=\"ugc\" data-user=\"{}\">@<span>{}</span>@<span>{}</span></a>", user.ap_id, user.id.to_string(), user.name, user.instance));
        mentions.push(user);
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
        published: Utc::now(),
        local_only: options.local_only,
        visibility: options.visibility,
        content_warning: options.content_warning,
    };

    let mentions_data: Vec<PostMention> = mentions
        .iter()
        .map(move |mention| PostMention {
            id: DbId::default(),
            post_id: id.clone(),
            mentioned_user_id: mention.id.clone(),
        })
        .collect();

    if !options.local_only {
        let activity = CreateNote::from(ApNote(object.clone()).into_json(data).await?);

        let mut inboxes = by.reached_inboxes(&data.db_pool).await?;
        inboxes.extend(
            mentions
                .iter()
                .map(|mention| mention.shared_inbox_or_inbox().to_string()),
        );

        send_activity(
            activity,
            &ApUser(by.clone()),
            inboxes
                .into_iter()
                .map(|inbox| Url::parse(&inbox))
                .collect::<Result<Vec<Url>, url::ParseError>>()?,
            data,
        )
        .await?;
    }

    insert_into(posts::dsl::posts)
        .values(vec![object.clone()])
        .execute(&mut conn)
        .await?;

    insert_into(post_mention::dsl::post_mention)
        .values(mentions_data)
        .on_conflict((
            post_mention::dsl::post_id,
            post_mention::dsl::mentioned_user_id,
        ))
        .do_nothing()
        .execute(&mut conn)
        .await?;

    notifications::process_post(&object, &data.db_pool).await?;

    Ok(object)
}

pub async fn boost(
    user: &User,
    post: &Post,
    visibility: DbVisibility,
    data: &Data<Arc<AppState>>,
) -> anyhow::Result<PostBoost> {
    let mut conn = data.db_pool.get().await?;
    let id = DbId::default();
    let ap_id = format!("{}/activities/boosts/{}", user.ap_id, id.to_string());

    let object = PostBoost {
        id,
        ap_id,
        post_id: post.id.clone(),
        actor_id: user.id.clone(),
        visibility,
        published: Utc::now(),
    };

    if !post.local_only {
        let activity = ApAnnounce(object.clone()).into_json(data).await?;

        let user = ApUser(user.clone());
        let mut inboxes = user.reached_inboxes(&data.db_pool).await?;
        let author_inbox = user.shared_inbox_or_inbox().to_string();
        if !inboxes.contains(&author_inbox) {
            inboxes.push(author_inbox);
        }

        send_activity(
            activity,
            &user,
            inboxes
                .into_iter()
                .map(|inbox| Url::parse(&inbox))
                .collect::<Result<Vec<Url>, url::ParseError>>()?,
            data,
        )
        .await?;
    }

    let object_db = insert_into(post_boost::dsl::post_boost)
        .values(vec![object])
        .on_conflict((post_boost::actor_id, post_boost::post_id))
        .do_nothing()
        .get_result::<PostBoost>(&mut conn)
        .await?;

    notifications::process_boost(post, user, false, &data.db_pool).await?;

    Ok(object_db)
}

pub async fn like(user: &User, post: &Post, data: &Data<Arc<AppState>>) -> anyhow::Result<()> {
    let mut conn = data.db_pool.get().await?;
    let id = Url::parse(&format!(
        "{}/activities/likes/{}",
        user.ap_id,
        DbId::default().to_string()
    ))?;
    let activity = Like {
        id: id.clone(),
        kind: Default::default(),
        actor: ObjectId::<ApUser>::from(Url::parse(&user.ap_id)?),
        object: ObjectId::<ApNote>::from(Url::parse(&post.ap_id)?),
    };

    let inboxes = vec![ApUser(post.author(&data.db_pool).await?).shared_inbox_or_inbox()];
    send_activity(activity, &ApUser(user.clone()), inboxes, data).await?;

    insert_into(post_like::dsl::post_like)
        .values(vec![PostLike {
            actor_id: user.id.clone(),
            post_id: post.id.clone(),
            ap_id: id.to_string(),
            published: Utc::now(),
        }])
        .on_conflict((post_like::actor_id, post_like::post_id))
        .do_nothing()
        .execute(&mut conn)
        .await?;

    notifications::process_like(post, user, false, &data.db_pool).await?;

    Ok(())
}

pub async fn unlike(user: &User, post: &Post, data: &Data<Arc<AppState>>) -> anyhow::Result<()> {
    let mut conn = data.db_pool.get().await?;
    let undo_id = Url::parse(&format!(
        "{}/activities/undo/likes/{}",
        user.ap_id,
        DbId::default().to_string()
    ))?;
    let like_id = post_like::table
        .select(post_like::ap_id)
        .filter(post_like::actor_id.eq(user.id.clone()))
        .filter(post_like::post_id.eq(post.id.clone()))
        .first::<String>(&mut conn)
        .await?;

    let activity = UndoLike {
        actor: ObjectId::<ApUser>::from(Url::parse(&user.ap_id)?),
        object: Like {
            id: Url::parse(&like_id)?,
            kind: Default::default(),
            actor: ObjectId::<ApUser>::from(Url::parse(&user.ap_id)?),
            object: ObjectId::<ApNote>::from(Url::parse(&post.ap_id)?),
        },
        kind: Default::default(),
        id: undo_id,
    };

    let inboxes = vec![ApUser(post.author(&data.db_pool).await?).shared_inbox_or_inbox()];
    send_activity(activity, &ApUser(user.clone()), inboxes, data).await?;

    let _ = delete(
        post_like::table
            .filter(post_like::actor_id.eq(user.id.clone()))
            .filter(post_like::post_id.eq(post.id.clone())),
    )
    .execute(&mut conn)
    .await;

    notifications::process_like(post, user, true, &data.db_pool).await?;

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
