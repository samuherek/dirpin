-- Add migration script here
create table if not exists entries (
    id text primary key,
    note text not null,             -- tag name of the entry, shown as title
    data text,                      -- extra data for the entry. Makrdown, script, notes,...
    kind text not null,             -- enum of possible entry types
    hostname text not null,         -- readable hostname reference for a unique computer
    cwd text not null,
    cgd text,                       -- current git directory
    created_at integer not null,
    updated_at integer not null,    -- timestamp to use for syncing
    deleted_at integer,
    version integer not null        -- sync version
);

create index if not exists idx_entries_updated_at on entries(updated_at);
