-- Add migration script here
create table if not exists pins(
    id integer primary key,         -- internal id for this db
    client_id text not null unique, -- the id of the item on the client
    user_id integer not null,       -- id of the registered user
    timestamp integer not null,     -- nonencryped metadata to konw the last update
    version integer not null,       -- nonencryped metadata to know the latest update
    data text not null,             -- encrypted data for the pin
    created_at integer not null
);

create index if not exists idx_pins_timestamp on pins(timestamp);