use crate::rpc::artalk::{page_comment, page_pv};
use crate::views::bbs::FullIssue;
use dtiku_bbs::model::{Issue, IssueQuery};
use spring::plugin::service::Service;
use spring_sea_orm::pagination::{Page, Pagination};
use spring_sea_orm::DbConn;

#[derive(Clone, Service)]
pub struct IssueService {
    #[inject(component)]
    db: DbConn,
}

impl IssueService {
    pub async fn find_issue_by_id(&self, id: i32) -> anyhow::Result<Option<FullIssue>> {
        let issue = Issue::find_issue_by_id(&self.db, id).await?;
        match issue {
            Some(i) => {
                let key = format!("/bbs/issue/{}", i.id);
                let page_keys = vec![key];
                let pv = page_pv(&page_keys).await;
                let cmt = page_comment(&page_keys).await;
                Ok(Some(FullIssue::new(i, &pv, &cmt)))
            }
            None => Ok(None),
        }
    }

    pub async fn search(
        &self,
        query: &IssueQuery,
        pagination: &Pagination,
    ) -> anyhow::Result<Page<FullIssue>> {
        let issues = Issue::search(&self.db, query, pagination).await?;
        if issues.is_empty() {
            return Ok(Page::new(Vec::new(), pagination, issues.total_elements));
        }
        let page_keys = issues
            .iter()
            .map(|i| format!("/bbs/issue/{}", i.id))
            .collect();
        let pv = page_pv(&page_keys).await;
        let cmt = page_comment(&page_keys).await;
        Ok(issues.map(|i| FullIssue::new(i, &pv, &cmt)))
    }
}
