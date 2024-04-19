-- Your SQL goes here

CREATE TABLE received_activities (
    ap_id text primary key unique,
    published timestamptz not null default now()
);
