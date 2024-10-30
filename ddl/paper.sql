create extension if not exists vector;
create table if not exists label (
    id serial primary key,
    name varchar(32) not null,
    pid integer not null
);
create table if not exists paper(
    id serial primary key,
    title varchar(255) not null,
    descrp text default null,
    label_id integer not null,
    extra jsonb not null
);
create table if not exists question(
    id serial primary key,
    content text not null,
    extra jsonb not null,
    embedding vector(512) not null
);
create table if not exists paper_question (
    paper_id integer not null,
    question_id integer not null,
    sort smallint not null
);
create table if not exists material (
    id serial primary key,
    content text not null,
    embedding vector(512) not null
);
create table if not exists paper_material (
    paper_id integer not null,
    material_id integer not null,
    sort smallint not null
);
create table if not exists question_material (
    question_id integer not null,
    material_id integer not null
);
create table if not exists solution (
    id serial primary key,
    question_id integer not null,
    extra jsonb not null
);