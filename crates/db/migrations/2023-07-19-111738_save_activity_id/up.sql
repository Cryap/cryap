-- Your SQL goes here

ALTER TABLE user_followers ADD ap_id varchar(200) unique;
ALTER TABLE user_followers ADD published timestamp not null default now();

ALTER TABLE user_follow_requests ADD ap_id varchar(200) unique;
ALTER TABLE user_follow_requests ADD published timestamp not null default now();