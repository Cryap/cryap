-- Your SQL goes here

create table applications (
  id char(27) primary key unique,
  name varchar(100) not null,
  website varchar(200),
  redirect_uri varchar(200) not null,
  client_id char(32) not null unique,
  client_secret char(32) not null unique,
  published timestamp not null default now()
);
