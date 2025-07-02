create type idiom_type as enum('idiom', 'word');

create table if not exists idiom(
    id serial primary key,
    text varchar(6) not null,
    ty idiom_type not null,
    content jsonb default null,
    created timestamp not null,
    modified timestamp not null
);

create table if not exists idiom_ref(
    id serial primary key,
    idiom_id int not null,
    question_id int not null,
    paper_id int not null,
    label_id int not null,
    exam_id int2 not null,
    paper_type int2 not null,
    ty idiom_type not null
);

create index concurrently if not exists idx_idiom_ref_label_id_idiom_id on idiom_ref (label_id, idiom_id);

create materialized view idiom_ref_stats as
select
  label_id,
  idiom_id,
  count(distinct question_id) as question_count,
  count(distinct paper_id) as paper_count
from idiom_ref
group by label_id, idiom_id;
