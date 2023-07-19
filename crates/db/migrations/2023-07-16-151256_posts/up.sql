-- Your SQL goes here

CREATE TYPE visibility AS ENUM ('public', 'unlisted', 'private', 'direct');

create table posts (
  id char(27) primary key unique,
  author char(27) not null REFERENCES users(id),
  ap_id varchar(200) not null unique,
  local_only bool not null,
  content_warning text,
  content text not null,
  sensitive bool not null,
  in_reply char(27) REFERENCES posts(id),
  published timestamp not null, -- if post don't have published it will be a now(),
  updated timestamp,
  url varchar(200) not null, -- if url isn't exists in Activity it will be a ap_id 
  quote char(27) REFERENCES posts(id),
  visibility visibility not null
);

create table post_mention (
  id varchar(27) PRIMARY KEY,
  post_id char(27) NOT NULL REFERENCES posts(id),
  mentioned_user_id char(27) NOT NULL REFERENCES users(id),
  unique (post_id, mentioned_user_id)
)
