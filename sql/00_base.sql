create table if not exists system_config(
    id serial primary key,
    version integer not null default 1,
    key varchar(32) not null,
    value jsonb not null,
    created timestamp not null,
    modified timestamp not null
);
create table if not exists schedule_task(
    id serial primary key,
    version integer not null default 1,
    ty varchar(32) not null,
    active bool not null,
    context jsonb not null,
    run_count integer not null,
    instances jsonb not null,
    created timestamp not null,
    modified timestamp not null
);