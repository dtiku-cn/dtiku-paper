create extension if not exists vector;
create extension if not exists ltree;
create extension if not exists pg_trgm;

CREATE TYPE from_type AS ENUM (
	'fenbi',
	'huatu',
	'offcn',
	'chinagwy'
);
create type src_type as enum('question', 'material', 'solution');
-- 考试类型：root_id为exam_id; leaf_id为paper_type
drop table if exists exam_category;
create table if not exists exam_category(
    id serial2 primary key,
    name varchar(32) not null,
    prefix varchar(32) not null,
    pid int2 not null,
    from_ty from_type not null,
    unique(from_ty, pid, prefix)
);
-- 试卷标签：比如省、市；
drop table if exists label;
create table if not exists label(
    id serial primary key,
    name varchar(32) not null,
    pid integer not null,
    exam_id int2 not null,
    paper_type int2 not null,
    unique(paper_type, pid, name)
);
-- 试卷
drop table if exists paper;
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
create index if not exists idx_paper_title_trgm
on paper
using gin (title gin_trgm_ops);
-- 知识点
drop table if exists key_point;
create table if not exists key_point(
    id serial primary key,
    name varchar(64) not null,
    pid integer not null,
    exam_id int2 not null,
    paper_type int2 not null,
    unique(paper_type, pid, name)
);
-- 问题
drop table if exists question;
create table if not exists question(
    id serial primary key,
    content text not null,
    exam_id int2 not null,
    paper_type int2 not null,
    extra jsonb not null,
    embedding vector(768) not null
);
drop table if exists question_key_point;
create table if not exists question_key_point(
    question_id integer not null,
    key_point_id integer not null,
    year int2 not null,
    primary key (question_id, key_point_id)
);
create index concurrently if not exists idx_qkp_for_agg
on question_key_point (key_point_id, year, question_id);
create materialized view if not exists question_key_point_stats as
select
    key_point_id,
    year,
    count(distinct question_id) as question_count
from
    question_key_point
group by
    key_point_id,
    year;

drop table if exists paper_question;
create table if not exists paper_question (
    paper_id integer not null,
    question_id integer not null,
    sort smallint not null,
    paper_type int2 not null,
    keypoint_path ltree default null,
    correct_ratio float4 default null,
    primary key (paper_id, question_id)
);
-- 材料
drop table if exists material;
create table if not exists material (
    id serial primary key,
    content text not null,
    extra jsonb not null
);
drop table if exists paper_material;
create table if not exists paper_material (
    paper_id integer not null,
    material_id integer not null,
    sort smallint not null,
    primary key (paper_id, material_id)
);
drop table if exists question_material;
create table if not exists question_material (
    question_id integer not null,
    material_id integer not null,
    primary key (question_id, material_id)
);
-- 解答
drop table if exists solution;
create table if not exists solution (
    id serial primary key,
    question_id integer not null,
    extra jsonb not null
);
--  图片,可能包含音频
drop table if exists assets;
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