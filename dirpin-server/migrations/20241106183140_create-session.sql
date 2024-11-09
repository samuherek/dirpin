-- Add migration script here
create table if not exists sessions (
	id integer primary key,
	user_id integer not null references users(id),
	token text unique not null,
    host_id text not null,                              -- random device id for device sessions
    expires_at integer not null
);

create index if not exists session_unique_idx on sessions(token);
