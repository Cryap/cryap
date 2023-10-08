-- Your SQL goes here

create table private_notes (
  id char(27) primary key unique,
  actor_id char(27) not null REFERENCES users(id),
  user_id char(27) not null REFERENCES users(id),
  note varchar(2000) not null,
  unique (actor_id, user_id)
)
