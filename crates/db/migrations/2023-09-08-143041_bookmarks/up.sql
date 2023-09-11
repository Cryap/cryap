-- Your SQL goes here

alter table post_like add published timestamp not null default now();

create table bookmarks (
  id char(27) primary key unique,
  post_id char(27) not null REFERENCES posts(id),
  actor_id char(27) not null REFERENCES users(id),
  published timestamp not null,
  unique (post_id, actor_id)
)
