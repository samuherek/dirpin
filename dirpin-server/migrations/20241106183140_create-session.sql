-- Add migration script here
create table if not exists sessions (
	id integer primary key,
	user_id integer not null,
	token text unique not null
);
