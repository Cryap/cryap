-- Your SQL goes here

ALTER TABLE users ADD followers_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD following_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD follow_requests_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD posts_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD last_post_published TIMESTAMPTZ;

UPDATE users 
SET 
    followers_count = (
        SELECT COUNT(*) FROM user_followers WHERE follower_id = users.id
    ),
    following_count = (
        SELECT COUNT(*) FROM user_followers WHERE actor_id = users.id
    ),
    follow_requests_count = (
        SELECT COUNT(*) FROM user_follow_requests WHERE follower_id = users.id
    ),
    posts_count = (
        (SELECT COUNT(*) FROM posts WHERE author = users.id AND visibility <> 'direct') +
        (SELECT COUNT(*) FROM post_boost WHERE actor_id = users.id AND visibility <> 'direct')
    ),
    last_post_published = (
        SELECT published FROM posts WHERE author = users.id AND visibility <> 'direct' UNION SELECT published FROM post_boost WHERE actor_id = users.id AND visibility <> 'direct' ORDER BY published DESC LIMIT 1
    );

-- followers_count and following_count

CREATE OR REPLACE FUNCTION handle_follow()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE users SET followers_count = followers_count + 1 WHERE id = NEW.follower_id;
    UPDATE users SET following_count = following_count + 1 WHERE id = NEW.actor_id;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER follow_trigger
AFTER INSERT ON user_followers
FOR EACH ROW
EXECUTE FUNCTION handle_follow();

CREATE OR REPLACE FUNCTION handle_unfollow()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE users SET followers_count = followers_count - 1 WHERE id = OLD.follower_id;
    UPDATE users SET following_count = following_count - 1 WHERE id = OLD.actor_id;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER unfollow_trigger
AFTER DELETE ON user_followers
FOR EACH ROW
EXECUTE FUNCTION handle_unfollow();

-- follow_requests_count

CREATE OR REPLACE FUNCTION handle_follow_request()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE users SET follow_requests_count = follow_requests_count + 1 WHERE id = NEW.follower_id;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER follow_request_trigger
AFTER INSERT ON user_follow_requests
FOR EACH ROW
EXECUTE FUNCTION handle_follow_request();

CREATE OR REPLACE FUNCTION handle_follow_request_deletion()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE users SET follow_requests_count = follow_requests_count - 1 WHERE id = OLD.follower_id;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER follow_request_deletion_trigger
AFTER DELETE ON user_follow_requests
FOR EACH ROW
EXECUTE FUNCTION handle_follow_request_deletion();

-- posts_count and last_post_published

CREATE OR REPLACE FUNCTION handle_post()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.visibility = 'direct' THEN
        RETURN NULL;
    END IF;

    UPDATE users SET posts_count = posts_count + 1 WHERE id = NEW.author;
    UPDATE users SET last_post_published = NEW.published WHERE id = NEW.author AND (last_post_published IS NULL OR NEW.published > last_post_published);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER post_trigger
AFTER INSERT ON posts
FOR EACH ROW
EXECUTE FUNCTION handle_post();

CREATE OR REPLACE FUNCTION handle_post_boost()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.visibility = 'direct' THEN
        RETURN NULL;
    END IF;

    UPDATE users SET posts_count = posts_count + 1 WHERE id = NEW.actor_id;
    UPDATE users SET last_post_published = NEW.published WHERE id = NEW.actor_id AND (last_post_published IS NULL OR NEW.published > last_post_published);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER post_boost_trigger
AFTER INSERT ON post_boost
FOR EACH ROW
EXECUTE FUNCTION handle_post_boost();
