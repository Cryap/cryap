-- This file should undo anything in `up.sql`

ALTER TABLE post_like
    DROP CONSTRAINT post_like_pkey,
    ADD PRIMARY KEY (ap_id),
    ALTER COLUMN ap_id SET NOT NULL;
