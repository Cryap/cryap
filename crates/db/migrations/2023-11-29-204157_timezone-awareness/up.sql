-- Your SQL goes here

SET timezone = 'UTC';

ALTER TABLE applications
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE bookmarks
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE notifications
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE post_boost
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE post_like
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE post_like
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE posts
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE posts
    ALTER COLUMN updated TYPE timestamptz
    USING updated;

ALTER TABLE sessions
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE user_follow_requests
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE user_followers
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE users
    ALTER COLUMN published TYPE timestamptz
    USING published;

ALTER TABLE users
    ALTER COLUMN updated TYPE timestamptz
    USING updated;
