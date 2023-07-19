-- Your SQL goes here

create table post_like (
  ap_id varchar(200) PRIMARY KEY,
  post_id char(27)  NOT NULL REFERENCES posts(id),
  actor_id char(27) NOT NULL REFERENCES users(id),
  unique (post_id, actor_id)
);

create table post_boost (
  ap_id varchar(200) PRIMARY KEY,
  post_id char(27)  NOT NULL REFERENCES posts(id),
  actor_id char(27) NOT NULL REFERENCES users(id),
  unique (post_id, actor_id)
)
