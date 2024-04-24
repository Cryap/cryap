-- Your SQL goes here

ALTER TABLE post_like
    DROP CONSTRAINT post_like_pkey,
    ADD PRIMARY KEY (post_id, actor_id),
    ALTER COLUMN ap_id DROP NOT NULL;