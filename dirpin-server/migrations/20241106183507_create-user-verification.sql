-- Add migration script here
create table if not exists user_verification_token(
  id integer primary key, 
  user_id integer unique references users(id), 
  token text not null, 
  expires_at integer not null
);
