-- Add migration script here
-- This is a copy of an entry table but we store the remote version.
-- Not sure if this is the right way to go about it. 
create table if not exists conflicts (
    id text primary key,
    value text not null,            
    data text,                      
    kind text not null,             
    hostname text not null,         
    cwd text not null,
    cgd text,                       
    created_at integer not null,
    updated_at integer not null,    
    deleted_at integer,
    version integer not null        
);
