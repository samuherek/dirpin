-- Add migration script here
create table if not exists entries(
    id integer primary key,         -- internal id for this db
    client_id text not null unique, -- the id of the item on the client
    user_id integer not null,       -- id of the registered user
    version integer not null,       -- nonencryped metadata to know the latest update
    data text not null,             -- encrypted data for the pin
    updated_at integer not null,    -- nonencryped metadata to konw the last update
    created_at integer not null,    -- Tag when this was created on the server
    deleted_at                      -- Soft delete
);

create index if not exists idx_entries_updated_at on entries(updated_at);
create index if not exists idx_entries_deleted_at on entries(deleted_at);
 
