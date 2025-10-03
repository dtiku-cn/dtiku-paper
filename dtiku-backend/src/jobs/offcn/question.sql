--- question type
WITH ranked_samples AS (
    SELECT
        extra->>'type' AS type,
        extra->>'form' AS form,
        extra,
        ROW_NUMBER() OVER (
            PARTITION BY extra->>'type', extra->>'form'
            ORDER BY RANDOM()
        ) AS row_num
    FROM question
    WHERE from_ty = 'offcn'
)
SELECT
    type,
    form,
    extra
FROM ranked_samples
WHERE row_num <= 10
ORDER BY type, form, row_num;