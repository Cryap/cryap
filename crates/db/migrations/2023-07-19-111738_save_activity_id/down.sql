-- This file should undo anything in `up.sql`

ALTER TABLE user_followers DROP COLUMN ap_id;
ALTER TABLE user_followers DROP COLUMN published;

ALTER TABLE user_follow_requests DROP COLUMN ap_id;
ALTER TABLE user_follow_requests DROP COLUMN published;