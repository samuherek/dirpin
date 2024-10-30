-- Add migration script here
create table if not exists pins (
    id text primary key,
    data text not null,
    hostname text not null,
    cwd text not null,
    cgd text,
    created_at integer not null,
    updated_at integer not null,
    deleted_at integer
);

create index if not exists idx_pins_updated_at on pins(updated_at);
