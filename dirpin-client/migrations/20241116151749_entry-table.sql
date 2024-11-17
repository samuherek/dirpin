-- Add migration script here
create table if not exists entries (
    id text primary key,
    value text not null,            -- name of the entry, shown as title
    desc text,                      -- short tag description if large data not needed
    data text,                      -- extra data for the entry. Makrdown, script, notes,...
    kind text not null,             -- enum of possible entry types
    path text not null,
    updated_at integer not null,    -- timestamp to use for syncing
    deleted_at integer,             -- if timestamp than it's soft deleted
    version integer not null,       -- sync version
    workspace_id text,              -- if part of a workspace
    host_id text not null,          -- host identifier for filtering

    foreign key(workspace_id) references workspaces(id)
);
