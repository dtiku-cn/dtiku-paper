--- question type
with ranked_samples as (
    select
        extra->>'type' AS type,
        extra,
        row_number() over (partition by extra->>'type' order by random()) as row_num
    from question
    where from_ty='offcn'
)
select
    type,
    extra
from ranked_samples
where row_num <= 10
order by type, row_num;