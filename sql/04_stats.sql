create type idiom_type as enum('idiom', 'word');

create table if not exists idiom(
    id serial primary key,
    text varchar(6) not null,
    ty idiom_type not null,
    content jsonb default null,
    created timestamp not null,
    modified timestamp not null
);