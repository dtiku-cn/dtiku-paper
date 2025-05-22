use crate::domain::issue::FullIssue;
use crate::http::artalk::{page_comment, page_pv};
use crate::model::{Issue, IssueQuery};
use anyhow::Context;
use sea_orm::EntityTrait;
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
        let issue = Issue::find_by_id(id)
            .one(&self.db)
            .await
            .with_context(|| format!("Issue::find_by_id({id}) failed"))?;
        match issue {
            Some(i) => {
                let key = format!("/bbs/issue/{}", i.id);
                let page_keys = vec![key];
                let pv = page_pv(&page_keys).await;
                let cmt = page_comment(&page_keys).await;
                Ok(Some(i.to_full_issue(&pv, &cmt)))
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
        Ok(issues.map(|i| i.to_full_issue(&pv, &cmt)))
    }
}
