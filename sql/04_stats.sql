create type idiom_type as enum('idiom', 'word');

create table if not exists idiom(
    id serial primary key,
    text varchar(6) not null,
    explain varchar(255) not null,
    ty idiom_type not null,
    content jsonb not null,
    created timestamp not null,
    modified timestamp not null,
    unique(text)
);

create table if not exists idiom_ref(
    id serial primary key,
    idiom_id int not null,
    question_id int not null,
    paper_id int not null,
    label_id int not null,
    exam_id int2 not null,
    paper_type int2 not null,
    ty idiom_type not null,
    unique(ty, label_id, idiom_id, paper_id, question_id)
);

create index if not exists idx_idiom_ref_idiom_id on idiom_ref(idiom_id);

create materialized view idiom_ref_stats as
select
  ty,
  label_id,
  idiom_id,
  count(distinct question_id) as question_count,
  count(distinct paper_id) as paper_count
from idiom_ref
group by ty, label_id, idiom_id;
