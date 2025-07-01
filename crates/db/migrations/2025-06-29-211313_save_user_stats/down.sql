-- This file should undo anything in `up.sql`

ALTER TABLE users DROP COLUMN followers_count;
ALTER TABLE users DROP COLUMN following_count;
ALTER TABLE users DROP COLUMN follow_requests_count;
ALTER TABLE users DROP COLUMN posts_count;
ALTER TABLE users DROP COLUMN last_post_published;

DROP FUNCTION handle_follow CASCADE;
DROP FUNCTION handle_unfollow CASCADE;
DROP FUNCTION handle_follow_request CASCADE;
DROP FUNCTION handle_follow_request_deletion CASCADE;
DROP FUNCTION handle_post CASCADE;
DROP FUNCTION handle_post_boost CASCADE;
