use spring_sea_orm::pagination::Pagination;

#[derive(Debug)]
pub struct ListPaperQuery {
    pub paper_type: i16,
    pub label_id: i32,
    pub page: Pagination,
}
