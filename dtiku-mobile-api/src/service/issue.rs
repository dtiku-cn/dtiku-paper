use dtiku_bbs::model::{issue, IssueQuery};
use spring::plugin::service::Service;
use spring_sea_orm::pagination::{Page, Pagination};
use spring_sea_orm::DbConn;

#[derive(Clone, Service)]
pub struct IssueService {
    #[inject(component)]
    db: DbConn,
}

impl IssueService {
    #[allow(dead_code)]
    pub async fn find_issue_by_id(&self, id: i32) -> anyhow::Result<Option<issue::Model>> {
        issue::Entity::find_issue_by_id(&self.db, id).await
    }

    pub async fn search(
        &self,
        query: &IssueQuery,
        pagination: &Pagination,
    ) -> anyhow::Result<Page<issue::Model>> {
        let issues = issue::Entity::search(&self.db, query, pagination)
            .await?
            .map(|list_issue| issue::Model {
                id: list_issue.id,
                user_id: list_issue.user_id,
                topic: list_issue.topic,
                pin: list_issue.pin,
                collect: list_issue.collect,
                created: list_issue.created,
                modified: list_issue.modified,
                title: list_issue.title,
                markdown: String::new(),
                html: String::new(),
            });
        Ok(issues)
    }
}

