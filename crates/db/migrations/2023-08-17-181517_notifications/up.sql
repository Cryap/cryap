-- Your SQL goes here

CREATE TYPE notification_type AS ENUM ('mention', 'reblog', 'follow', 'follow_request', 'favourite', 'quote');

create table notifications (
  id char(27) primary key unique,
  actor_id char(27) not null REFERENCES users(id),
  receiver_id char(27) not null REFERENCES users(id),
  post_id char(27) REFERENCES posts(id),
  notification_type notification_type not null,
  published timestamp not null
);
