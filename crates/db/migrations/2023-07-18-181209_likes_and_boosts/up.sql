-- Your SQL goes here

create table post_like (
  ap_id varchar(200) primary key,
  post_id char(27) not null REFERENCES posts(id),
  actor_id char(27) not null REFERENCES users(id),
  unique (post_id, actor_id)
);

create table post_boost (
  id char(27) primary key unique,
  ap_id varchar(200) not null unique,
  post_id char(27) not null REFERENCES posts(id),
  actor_id char(27) not null REFERENCES users(id),
  visibility visibility not null,
  published timestamp not null,
  unique (post_id, actor_id)
)
