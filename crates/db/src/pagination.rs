use serde::Deserialize;

#[macro_export]
macro_rules! paginate {
    ( $query:expr, $column:path, $pagination:expr ) => {{
        let query = $query;
        match $pagination {
            Pagination::MaxId(id, limit) => query.filter($column.gt(id)).limit(limit.into()),
            Pagination::MinId(id, limit) => query.filter($column.lt(id)).limit(limit.into()),
            Pagination::None(limit) => query.limit(limit.into()),
        }
    }};
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub max_id: Option<String>,
    pub min_id: Option<String>,
    pub since_id: Option<String>,
    pub limit: Option<i32>,
}

pub enum Pagination {
    MaxId(String, i32),
    MinId(String, i32),
    None(i32),
}

impl From<PaginationQuery> for Pagination {
    fn from(value: PaginationQuery) -> Self {
        let limit = match value.limit {
            None => 20,
            Some(limit) if limit < 40 => limit,
            _ => 40,
        };

        if let Some(max_id) = value.max_id {
            Pagination::MaxId(max_id, limit)
        } else if let Some(min_id) = value.min_id.or(value.since_id) {
            // In fact, min_id and since_id in Mastodon mean the same thing
            Pagination::MinId(min_id, limit)
        } else {
            Pagination::None(limit)
        }
    }
}
