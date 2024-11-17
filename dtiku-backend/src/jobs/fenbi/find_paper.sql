select 
    jsonb_extract_path_text(extra,'name') as name,
    jsonb_extract_path_text(extra,'date') as date,
    jsonb_extract_path_text(extra,'topic') as topic,
    jsonb_extract_path_text(extra,'type') as ty,
    jsonb_extract_path_text(extra,'chapters') as chapters,
    id
from paper p 
where from_ty ='fenbi'
and id > ?
and id <= ?