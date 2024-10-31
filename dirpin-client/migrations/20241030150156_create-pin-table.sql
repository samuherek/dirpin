-- Add migration script here
create table if not exists pins (
    id text primary key,
    data text not null,
    hostname text not null,         -- readable hostname reference for a unique computer
    cwd text not null,
    cgd text,                       -- current git directory
    created_at integer not null,
    updated_at integer not null,    -- timestamp to use for syncing
    deleted_at integer,
    version integer not null        -- sync version
);

create index if not exists idx_pins_updated_at on pins(updated_at);
