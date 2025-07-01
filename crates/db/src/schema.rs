// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "notification_type"))]
    pub struct NotificationType;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "visibility"))]
    pub struct Visibility;
}

diesel::table! {
    applications (id) {
        #[max_length = 27]
        id -> Bpchar,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 200]
        website -> Nullable<Varchar>,
        #[max_length = 200]
        redirect_url -> Varchar,
        #[max_length = 32]
        client_id -> Bpchar,
        #[max_length = 32]
        client_secret -> Bpchar,
        published -> Timestamptz,
    }
}

diesel::table! {
    bookmarks (id) {
        #[max_length = 27]
        id -> Bpchar,
        #[max_length = 27]
        post_id -> Bpchar,
        #[max_length = 27]
        actor_id -> Bpchar,
        published -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::NotificationType;

    notifications (id) {
        #[max_length = 27]
        id -> Bpchar,
        #[max_length = 27]
        actor_id -> Bpchar,
        #[max_length = 27]
        receiver_id -> Bpchar,
        #[max_length = 27]
        post_id -> Nullable<Bpchar>,
        notification_type -> NotificationType,
        published -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Visibility;

    post_boost (id) {
        #[max_length = 27]
        id -> Bpchar,
        #[max_length = 200]
        ap_id -> Varchar,
        #[max_length = 27]
        post_id -> Bpchar,
        #[max_length = 27]
        actor_id -> Bpchar,
        visibility -> Visibility,
        published -> Timestamptz,
    }
}

diesel::table! {
    post_like (post_id, actor_id) {
        #[max_length = 200]
        ap_id -> Nullable<Varchar>,
        #[max_length = 27]
        post_id -> Bpchar,
        #[max_length = 27]
        actor_id -> Bpchar,
        published -> Timestamptz,
    }
}

diesel::table! {
    post_mention (id) {
        #[max_length = 27]
        id -> Varchar,
        #[max_length = 27]
        post_id -> Bpchar,
        #[max_length = 27]
        mentioned_user_id -> Bpchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Visibility;

    posts (id) {
        #[max_length = 27]
        id -> Bpchar,
        #[max_length = 27]
        author -> Bpchar,
        #[max_length = 200]
        ap_id -> Varchar,
        local_only -> Bool,
        content_warning -> Nullable<Text>,
        content -> Text,
        sensitive -> Bool,
        #[max_length = 27]
        in_reply -> Nullable<Bpchar>,
        published -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        #[max_length = 200]
        url -> Varchar,
        #[max_length = 27]
        quote -> Nullable<Bpchar>,
        visibility -> Visibility,
    }
}

diesel::table! {
    private_notes (id) {
        #[max_length = 27]
        id -> Bpchar,
        #[max_length = 27]
        actor_id -> Bpchar,
        #[max_length = 27]
        user_id -> Bpchar,
        #[max_length = 2000]
        note -> Varchar,
    }
}

diesel::table! {
    received_activities (ap_id) {
        ap_id -> Text,
        published -> Timestamptz,
    }
}

diesel::table! {
    sessions (id) {
        #[max_length = 27]
        id -> Bpchar,
        #[max_length = 60]
        token -> Bpchar,
        #[max_length = 27]
        user_id -> Bpchar,
        published -> Timestamptz,
        #[max_length = 27]
        application_id -> Nullable<Bpchar>,
    }
}

diesel::table! {
    user_follow_requests (actor_id, follower_id) {
        #[max_length = 27]
        actor_id -> Bpchar,
        #[max_length = 27]
        follower_id -> Bpchar,
        #[max_length = 200]
        ap_id -> Nullable<Varchar>,
        published -> Timestamptz,
    }
}

diesel::table! {
    user_followers (actor_id, follower_id) {
        #[max_length = 27]
        actor_id -> Bpchar,
        #[max_length = 27]
        follower_id -> Bpchar,
        #[max_length = 200]
        ap_id -> Nullable<Varchar>,
        published -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        #[max_length = 27]
        id -> Bpchar,
        #[max_length = 200]
        ap_id -> Varchar,
        local -> Bool,
        #[max_length = 200]
        inbox_uri -> Varchar,
        #[max_length = 200]
        shared_inbox_uri -> Nullable<Varchar>,
        #[max_length = 200]
        outbox_uri -> Varchar,
        #[max_length = 200]
        followers_uri -> Varchar,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 100]
        instance -> Varchar,
        #[max_length = 100]
        display_name -> Nullable<Varchar>,
        bio -> Nullable<Text>,
        password_encrypted -> Nullable<Text>,
        admin -> Bool,
        public_key -> Text,
        private_key -> Nullable<Text>,
        published -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        manually_approves_followers -> Bool,
        is_cat -> Bool,
        bot -> Bool,
        followers_count -> Int4,
        following_count -> Int4,
        follow_requests_count -> Int4,
        posts_count -> Int4,
        last_post_published -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(bookmarks -> posts (post_id));
diesel::joinable!(bookmarks -> users (actor_id));
diesel::joinable!(notifications -> posts (post_id));
diesel::joinable!(post_boost -> posts (post_id));
diesel::joinable!(post_boost -> users (actor_id));
diesel::joinable!(post_like -> posts (post_id));
diesel::joinable!(post_like -> users (actor_id));
diesel::joinable!(post_mention -> posts (post_id));
diesel::joinable!(post_mention -> users (mentioned_user_id));
diesel::joinable!(posts -> users (author));
diesel::joinable!(sessions -> applications (application_id));
diesel::joinable!(sessions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    applications,
    bookmarks,
    notifications,
    post_boost,
    post_like,
    post_mention,
    posts,
    private_notes,
    received_activities,
    sessions,
    user_follow_requests,
    user_followers,
    users,
);
