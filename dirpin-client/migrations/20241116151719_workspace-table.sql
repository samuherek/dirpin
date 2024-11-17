-- Add migration script here
create table if not exists workspaces (
    id text primary key,
    name text not null,
    git text,                               -- git remote reference
    paths text not null,                    -- list of paths across all hosts
    updated_at integer not null,            -- timestamp to use for syncing
    deleted_at integer,                     -- soft delete timestamp for syncing
    version integer not null                -- sync version
);
