use diesel::{
    sql_function,
    sql_types::{Nullable, Varchar},
};
use rand::{distributions::Alphanumeric, Rng};

pub fn random_string(size: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .map(char::from)
        .collect()
}

sql_function! { fn coalesce(x: Nullable<Varchar>, y: Varchar) -> Varchar; }
