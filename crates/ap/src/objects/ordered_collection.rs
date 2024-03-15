use activitypub_federation::kinds::kind;
use serde::{Deserialize, Serialize};

kind!(OrderedCollectionType, OrderedCollection);
kind!(OrderedCollectionPageType, OrderedCollectionPage);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollection {
    #[serde(rename = "type")]
    pub kind: OrderedCollectionType,
    pub id: String,
    pub total_items: i64,
    pub first: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollectionPage<T: Serialize> {
    #[serde(rename = "type")]
    pub kind: OrderedCollectionPageType,
    pub id: String,
    pub total_items: i64,
    pub next: Option<String>,
    pub prev: Option<String>,
    pub part_of: String,
    pub ordered_items: Vec<T>,
}
