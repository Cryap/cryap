-- This file should undo anything in `up.sql`

alter table post_like drop column published;
drop table bookmarks;