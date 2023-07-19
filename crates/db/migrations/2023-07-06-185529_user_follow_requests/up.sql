-- Your SQL goes here

create table user_follow_requests (
  actor_id char(27) REFERENCES users(id),
  follower_id char(27) REFERENCES users(id),

  PRIMARY KEY (actor_id, follower_id)
);
