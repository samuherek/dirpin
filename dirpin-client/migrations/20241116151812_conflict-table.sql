-- Add migration script here
create table if not exists conflicts (
    ref_id text primary key,                       -- The reference id to the table entry
    ref_kind text not null,                     -- Table kind to define correct parser of the data
    data text not null                      -- The actual stringified data for the ref_id
)
