-- Your SQL goes here

ALTER TABLE sessions ADD application_id char(27) REFERENCES applications(id);
