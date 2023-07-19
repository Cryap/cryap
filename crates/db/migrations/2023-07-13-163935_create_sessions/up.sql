-- Your SQL goes here

create table sessions (
  id char(27) primary key unique,
  token char(60) not null unique,
  user_id char(27) not null REFERENCES users(id),
  published timestamp not null default now()
);
