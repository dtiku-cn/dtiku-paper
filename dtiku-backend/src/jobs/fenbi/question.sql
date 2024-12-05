--- question type
with ranked_samples as (
    select
         (extra->>'type')::int AS type,
        extra,
        row_number() over (partition by extra->>'type' order by random()) as row_num
    from question
    where from_ty='fenbi'
)
select
    type,
    extra
from ranked_samples
where row_num <= 10
order by type, row_num;

--- options type
select 
    (accessory->>'type')::int as type,
    count(*) as count
from (
    select jsonb_array_elements(extra->'accessories') as accessory
    from question
    where from_ty='fenbi'
) as accessories_expanded
group by accessory->>'type'
order by count desc;

with expanded_accessories as (
    select
        extra,
        (accessory->>'type')::int as type
    from question, jsonb_array_elements(extra->'accessories') as accessory
    where from_ty='fenbi'
),
ranked_samples as (
    select
        extra,
        type,
        row_number() over (partition by type order by random()) as row_num
    from expanded_accessories
)
select
    type,
    extra
from ranked_samples
where row_num <= 10
order by type, row_num;

--- material
with expanded_material_accessories as (
    select
        extra,
        (material_accessory->>'type')::int as type
    from question, jsonb_array_elements(extra->'material'->'accessories') as material_accessory
    where from_ty='fenbi'
),
ranked_samples as (
    select
        extra,
        type,
        row_number() over (partition by type order by random()) as row_num
    from expanded_material_accessories
)
select
    type,
    extra
from ranked_samples
where row_num <= 10
order by type, row_num;

--- solution
WITH ranked_samples AS (
    SELECT
        (extra->'correctAnswer'->>'type')::int AS correct_answer_type,  -- 提取并转换 type 字段为 int
        extra,
        ROW_NUMBER() OVER (PARTITION BY (extra->'correctAnswer'->>'type')::int ORDER BY RANDOM()) AS row_num
    FROM question
    where from_ty='fenbi'
)
SELECT
    correct_answer_type,
    extra
FROM ranked_samples
WHERE row_num <= 10
ORDER BY correct_answer_type, row_num;

select 
extra->'solution',
extra->'correctAnswer',
 extra->'solutionAccessories',
extra
from question
where from_ty = 'fenbi'
and extra->'correctAnswer' != 'null'
and (extra->>'type')::int > 10
and extra->'solutionAccessories' != '[]'