-- Add migration script here
create table if not exists sessions (
	id integer primary key,
	user_id integer,
	token text unique not null
);
