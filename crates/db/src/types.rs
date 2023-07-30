use std::fmt;

use diesel_derive_newtype::DieselNewType;
use serde::{Deserialize, Serialize};
use svix_ksuid::KsuidLike;

#[derive(DieselNewType, Debug, Hash, PartialEq, Eq, Clone)]
pub struct DbId(String);

impl Default for DbId {
    fn default() -> Self {
        DbId(svix_ksuid::Ksuid::new(None, None).to_string())
    }
}

impl fmt::Display for DbId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for DbId {
    fn from(string: String) -> Self {
        DbId(string)
    }
}

impl From<svix_ksuid::Ksuid> for DbId {
    fn from(id: svix_ksuid::Ksuid) -> Self {
        DbId(id.to_string())
    }
}

#[derive(
    diesel_derive_enum::DbEnum, Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
#[ExistingTypePath = "crate::schema::sql_types::Visibility"]
pub enum DbVisibility {
    Public,
    Unlisted,
    Private,
    Direct,
}
