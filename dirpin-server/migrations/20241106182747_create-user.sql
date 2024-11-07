-- Add migration script here
create table if not exists users (
	id integer primary key,                 
	username text not null unique,          
	email text not null unique,             
	password text not null,
    verified_at integer not null,
    created_at integer not null
);

-- the prior index is case sensitive :(
create index if not exists email_unique_idx on users(LOWER(email));
create index if not exists username_unique_idx on users(LOWER(username));
