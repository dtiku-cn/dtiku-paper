create type topic_type as enum(
    'xingce',
    'shenlun',
    'mianshi',
    'shenhe',
    'linxuan',
    'sydw',
    'share',
    'growth'
);
create table if not exists issue(
    id serial primary key,
    topic topic_type not null,
    title varchar(255) not null,
    markdown text not null,
    html text not null,
    user_id int not null,
    pin bool not null default false,
    collect bool not null default false,
    created timestamp not null,
    modified timestamp not null
);