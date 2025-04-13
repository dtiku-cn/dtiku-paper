create extension if not exists vector;
create extension if not exists ltree;
-- 考试类型：root_id为exam_id; leaf_id为paper_type
create table if not exists exam_category(
    id serial2 primary key,
    name varchar(16) not null,
    prefix varchar(16) not null,
    pid int2 not null,
    unique(pid, prefix)
);
-- 试卷标签：比如省、市；
create table if not exists label(
    id serial primary key,
    name varchar(32) not null,
    pid integer not null,
    exam_id int2 not null,
    paper_type int2 not null,
    unique(paper_type, pid, name)
);
-- 试卷
create table if not exists paper(
    id serial primary key,
    title varchar(255) not null,
    year int2 not null,
    exam_id int2 not null,
    paper_type int2 not null,
    label_id integer not null,
    extra jsonb not null,
    unique(label_id, title)
);
-- 知识点
create table if not exists key_point(
    id serial primary key,
    name varchar(32) not null,
    pid integer not null,
    exam_id int2 not null,
    paper_type int2 not null
);
-- 问题
create table if not exists question(
    id serial primary key,
    content text not null,
    exam_id int2 not null,
    paper_type int2 not null,
    extra jsonb not null,
    embedding vector(768) not null
);
create table if not exists question_key_point(
    question_id integer not null,
    key_point_id integer not null,
    primary key (question_id, key_point_id)
);
create table if not exists paper_question (
    paper_id integer not null,
    question_id integer not null,
    sort smallint not null,
    keypoint_path ltree not null,
    correct_ratio float4 not null,
    primary key (paper_id, question_id)
);
-- 材料
create table if not exists material (
    id serial primary key,
    content text not null,
    extra jsonb not null
);
create table if not exists paper_material (
    paper_id integer not null,
    material_id integer not null,
    sort smallint not null,
    primary key (paper_id, material_id)
);
create table if not exists question_material (
    question_id integer not null,
    material_id integer not null,
    primary key (question_id, material_id)
);
-- 解答
create table if not exists solution (
    id serial primary key,
    question_id integer not null,
    extra jsonb not null
);
create type src_type as enum('question', 'material', 'solution');
--  图片,可能包含音频
create table if not exists assets(
    id serial primary key,
    src_type src_type not null,
    src_id integer not null,
    src_url text not null,
    storage_url text not null,
    created timestamp not null,
    modified timestamp not null,
    unique(src_type, src_id)
);