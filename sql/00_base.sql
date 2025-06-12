create table if not exists user_info (
    id serial primary key,
    wechat_id varchar(32) not null,
    name varchar(64) not null,
    gender bool not null,
    avatar varchar(255) not null,
    expired timestamp not null,
    created timestamp not null,
    modified timestamp not null
);
drop table if exists system_config;
create table if not exists system_config(
    id serial primary key,
    version integer not null default 1,
    key varchar(32) not null,
    value jsonb not null,
    created timestamp not null,
    modified timestamp not null,
    unique(key)
);
drop table if exists schedule_task;
create table if not exists schedule_task(
    id serial primary key,
    version integer not null default 1,
    ty varchar(32) not null,
    active bool not null,
    context jsonb not null,
    run_count integer not null,
    instances jsonb not null,
    created timestamp not null,
    modified timestamp not null,
    unique(ty)
);