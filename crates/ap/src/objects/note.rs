use std::sync::Arc;

use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::{kind, link::MentionType, object::NoteType},
    protocol::{helpers::deserialize_one_or_many, verification::verify_domains_match},
    traits::Object,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use db::{
    models::{Post, PostMention, User},
    schema::{post_mention, posts, users},
    types::{DbId, DbVisibility},
};
use diesel::{
    insert_into, query_dsl::QueryDsl, result::Error::NotFound, ExpressionMethods, SelectableHelper,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, NoneAsEmptyString};
use svix_ksuid::KsuidLike;
use url::Url;
use web::AppState;

use super::user::ApUser;
use crate::PUBLIC;

kind!(HashtagType, Hashtag);
kind!(EmojiType, Emoji);

db_to_ap!(db::models::Post, ApNote);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NoteTags {
    Mention(Mention),
    Hashtag(Hashtag),
    Emoji(Emoji),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Emoji {
    #[serde(rename = "type")]
    pub kind: EmojiType,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Mention {
    #[serde(rename = "type")]
    pub kind: MentionType,
    pub href: Url,
    name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hashtag {
    #[serde(rename = "type")]
    pub kind: HashtagType,
    pub href: Url,
    name: Option<String>,
}

pub fn parse_to_cc(to: &Vec<Url>, cc: &Vec<Url>, actor_followers_uri: Url) -> DbVisibility {
    let public_url = Url::parse(PUBLIC).unwrap();
    match (to, cc) {
        (_, _) if to.contains(&public_url) => DbVisibility::Public,
        (_, _) if cc.contains(&public_url) => DbVisibility::Unlisted,
        (_, _) if cc.contains(&actor_followers_uri) || to.contains(&actor_followers_uri) => {
            DbVisibility::Private
        },
        (_, _) => DbVisibility::Direct,
    }
}

pub fn construct_to_cc(
    visibility: &DbVisibility,
    followers_uri: Url,
    mention_ids: Vec<Url>,
) -> (Vec<Url>, Vec<Url>) {
    match visibility {
        DbVisibility::Public => (
            vec![Url::parse(PUBLIC).unwrap()],
            vec![followers_uri].into_iter().chain(mention_ids).collect(),
        ),
        DbVisibility::Unlisted => (
            vec![],
            vec![Url::parse(PUBLIC).unwrap(), followers_uri]
                .into_iter()
                .chain(mention_ids)
                .collect(),
        ),
        DbVisibility::Private => (vec![followers_uri], mention_ids),
        DbVisibility::Direct => (mention_ids, vec![]),
    }
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    #[serde(rename = "type")]
    pub kind: NoteType,
    pub id: ObjectId<ApNote>,
    pub attributed_to: ObjectId<ApUser>,

    pub content: String,
    pub url: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub summary: Option<String>,
    pub sensitive: Option<bool>,
    pub in_reply_to: Option<ObjectId<ApNote>>,
    pub published: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
    pub quote_uri: Option<ObjectId<ApNote>>,
    pub quote_url: Option<ObjectId<ApNote>>,
    #[serde(default)]
    pub tag: Vec<NoteTags>,

    #[serde(deserialize_with = "deserialize_one_or_many")]
    pub to: Vec<Url>,
    #[serde(deserialize_with = "deserialize_one_or_many", default)]
    pub cc: Vec<Url>,
}

impl ApNote {
    pub fn id(&self) -> Url {
        Url::parse(&self.ap_id).unwrap() // should never panic in theory
    }

    pub async fn into_json_mentions(
        self,
        data: &Data<Arc<AppState>>,
        mentions: &Vec<ApUser>,
    ) -> Result<Note, anyhow::Error> {
        if self.local_only {
            return Err(anyhow!("Cannot federate local-only object"));
        }

        let published = self.published;

        let attributed_to = match User::by_id(&self.author, &data.db_pool).await? {
            Some(n) => n,
            None => return Err(anyhow!("Post attributed to unknown user")),
        };

        let quote = match &self.quote {
            None => None,
            Some(quote) => match Post::by_id(quote, &data.db_pool).await? {
                Some(quote) => Some(ObjectId::<ApNote>::from(Url::parse(&quote.ap_id)?)),
                None => None,
            },
        };

        let in_reply_to = match &self.in_reply {
            None => None,
            Some(reply) => match Post::by_id(reply, &data.db_pool).await? {
                Some(reply) => Some(ObjectId::<ApNote>::from(Url::parse(&reply.ap_id)?)),
                None => None,
            },
        };

        // Panic safety: should never panic
        let mention_ids: Vec<Url> = mentions
            .iter()
            .map(|user| Url::parse(&user.ap_id).unwrap())
            .collect();

        let mut tags = vec![];

        for mention in mentions {
            tags.push(NoteTags::Mention(Mention {
                kind: Default::default(),
                href: Url::parse(&mention.ap_id)?,
                name: Some(match mention.instance == data.config.web.domain {
                    true => format!("@{}", mention.name),
                    false => format!("@{}@{}", mention.name, mention.instance),
                }),
            }))
        }

        let (to, cc) = construct_to_cc(
            &self.visibility,
            Url::parse(&attributed_to.followers_uri)?,
            mention_ids,
        );

        Ok(Note {
            kind: Default::default(),
            attributed_to: ObjectId::from(Url::parse(&attributed_to.ap_id)?),
            id: ObjectId::from(Url::parse(&self.ap_id)?),
            summary: self.content_warning.clone().or(Some("".to_string())),
            content: self.content.clone(),
            sensitive: Some(self.sensitive),
            url: Some(self.ap_id.clone()),
            in_reply_to,
            tag: tags,
            to,
            cc,
            quote_uri: quote.clone(), // AP moment
            quote_url: quote.clone(),
            published: Some(published),
            updated: self.updated,
        })
    }
}

#[async_trait::async_trait]
impl Object for ApNote {
    type DataType = Arc<AppState>;
    type Kind = Note;
    type Error = anyhow::Error;

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        let user = posts::table
            .filter(posts::ap_id.eq(object_id.to_string()))
            .first::<db::models::Post>(&mut data.db_pool.get().await?)
            .await;
        match user {
            Ok(post) => Ok(Some(ApNote(post))),
            Err(NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        if self.local_only {
            return Err(anyhow!("Cannot federate local-only object"));
        }

        let mentions: Vec<User> = post_mention::table
            .filter(post_mention::post_id.eq(self.id.clone()))
            .inner_join(users::table)
            .select(User::as_select())
            .load::<User>(&mut data.db_pool.get().await?)
            .await?;
        self.into_json_mentions(data, &mentions.into_iter().map(ApUser).collect())
            .await
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        if json.content.len() > 500_000 {
            return Err(anyhow!("Remote post is too big! 500k+ characters"));
        }
        if json.summary.clone().unwrap_or("".to_string()).len() > 1000 {
            return Err(anyhow!("Remote post CW is too big! 1k+ characters"));
        }
        // TODO: Check Hashtags and Mention limits
        Ok(())
    }

    async fn from_json(json: Self::Kind, data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
        let mut conn = data.db_pool.get().await?;
        let actor = json.attributed_to.dereference(data).await?;
        let reply = match json.in_reply_to {
            None => None,
            Some(ref reply) => Some(reply.dereference(data).await?.id.clone()),
        };
        let quote = match json.quote_uri {
            None => match json.quote_url {
                Some(ref quote) => Some(quote.dereference(data).await?.id.clone()),
                None => None,
            },
            Some(ref quote) => Some(quote.dereference(data).await?.id.clone()),
        };

        let post = Post {
            id: DbId::from(svix_ksuid::Ksuid::new(
                json.published
                    .map(|f| time::OffsetDateTime::from_unix_timestamp(f.timestamp()).unwrap()),
                None,
            )),
            author: actor.id.clone(),
            content: json.content, // TODO: sanitize
            url: json.url.unwrap_or(json.id.inner().to_string()),
            local_only: false, // remote post can't be local only
            visibility: parse_to_cc(&json.to, &json.cc, Url::parse(&actor.followers_uri)?),
            content_warning: json.summary,
            in_reply: reply,
            quote,
            sensitive: false,
            ap_id: json.id.to_string(),
            published: json.updated.unwrap_or(Utc::now()),
            updated: Some(Utc::now()),
        };

        let post_db = insert_into(posts::table)
            .values(post.clone())
            .on_conflict(posts::ap_id)
            .do_update()
            .set(post)
            .get_result::<Post>(&mut conn)
            .await?;

        let mut mentions: Vec<PostMention> = vec![];

        for tag in &json.tag {
            if let NoteTags::Mention(mention) = tag {
                mentions.push(PostMention {
                    id: DbId::default(),
                    post_id: post_db.id.clone(),
                    mentioned_user_id: ObjectId::<ApUser>::from(mention.href.clone())
                        .dereference(data)
                        .await?
                        .id
                        .clone(),
                })
            }
        }

        if !mentions.is_empty() {
            insert_into(post_mention::table)
                .values(mentions)
                .on_conflict((post_mention::post_id, post_mention::mentioned_user_id))
                .do_nothing()
                .execute(&mut conn)
                .await?;
        }

        Ok(ApNote(post_db))
    }
}
