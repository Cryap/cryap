-- Your SQL goes here

create table user_followers (
  actor_id char(27) REFERENCES users(id),
  follower_id char(27) REFERENCES users(id),

  PRIMARY KEY (actor_id, follower_id)
);
