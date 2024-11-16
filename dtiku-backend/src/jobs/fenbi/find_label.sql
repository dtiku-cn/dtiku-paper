select 
jsonb_extract_path_text(extra,'course_set','liveConfigItem','name') as exam_root,
jsonb_extract_path_text(extra,'course_set','liveConfigItem','prefix') as exam_root_prefix,
jsonb_extract_path_text(extra,'course_set','courseSet','name') as exam_name,
jsonb_extract_path_text(extra,'course_set','courseSet','prefix') as exam_prefix,
jsonb_extract_path_text(extra,'course','name') as paper_type,
jsonb_extract_path_text(extra,'course','prefix') as paper_prefix,
jsonb_extract_path_text(extra,'parent','name') as parent_label,
jsonb_extract_path_text(extra,'name') as label_name,
id
from label
where from_ty ='fenbi'
order by exam_root,exam_name,paper_type,parent_label,label_name