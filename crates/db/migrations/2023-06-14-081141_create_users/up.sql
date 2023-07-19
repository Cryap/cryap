-- Your SQL goes here

create table users (
  id char(27) primary key unique,

  ap_id varchar(200) not null unique,
  local bool not null,
  inbox_uri varchar(200) not null,
  shared_inbox_uri varchar(200),
  outbox_uri varchar(200) not null,
  followers_uri varchar(200) not null,

  -- profile
  name varchar(100) not null,
  instance varchar(100) not null,
  display_name varchar(100),
  bio text,
 
  -- auth
  password_encrypted text,
  admin boolean default false not null,
  public_key text not null,
  private_key text,

  -- times

  published timestamp not null default now(),
  updated timestamp,


  unique(name, instance)
);
