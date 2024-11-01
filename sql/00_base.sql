create table if not exists system_config(
    id serial primary key,
    version integer not null,
    key varchar(32) not null,
    value text not null,
    created timestamp not null,
    modified timestamp not null
);
create table if not exists schedule_task(
    id serial primary key,
    version integer not null,
    ty varchar(32) not null,
    active bool not null,
    context text not null,
    error_count integer not null,
    error_cause text not null,
    created timestamp not null,
    modified timestamp not null
);