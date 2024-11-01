create type topic_type as enum(
    'xingce',
    'shenlun',
    'mianshi',
    'shenhe',
    'linxuan',
    'sydw',
    'share',
    'work'
);
create table if not exists issue(
    id serial primary key,
    topic topic_type not null,
    title varchar(255) not null,
    markdown text not null,
    user_id bigint not null,
    created timestamp not null,
    modified timestamp not null
);