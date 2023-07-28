#[macro_export]
macro_rules! db_to_ap {
    ( $db:path, $ap:ident ) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $ap(pub $db);

        impl std::ops::Deref for $ap {
            type Target = $db;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<$db> for $ap {
            fn from(p: $db) -> Self {
                $ap(p)
            }
        }
    };
}

pub mod note;
pub mod service_actor;
pub mod user;
