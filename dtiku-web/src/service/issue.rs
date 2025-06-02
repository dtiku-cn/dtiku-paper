use crate::plugins::grpc_client::artalk::VoteStats;
use crate::plugins::grpc_client::Artalk;
use crate::rpc::artalk::{page_comment, page_pv};
use crate::views::bbs::FullIssue;
use dtiku_base::model::{user_info, UserInfo};
use dtiku_bbs::model::{Issue, IssueQuery};
use itertools::Itertools;
use spring::plugin::service::Service;
use spring_sea_orm::pagination::{Page, Pagination};
use spring_sea_orm::DbConn;
use std::collections::HashMap;

#[derive(Clone, Service)]
pub struct IssueService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    artalk: Artalk,
}

impl IssueService {
    pub async fn find_issue_by_id(&self, id: i32) -> anyhow::Result<Option<FullIssue>> {
        let issue = Issue::find_issue_by_id(&self.db, id).await?;
        match issue {
            Some(i) => {
                let u = UserInfo::find_user_by_id(&self.db, i.user_id).await?;
                let mut m = HashMap::new();
                if let Some(user) = u {
                    m.insert(user.id, user);
                }
                let key = format!("/bbs/issue/{}", i.id);
                let page_keys = vec![key.clone()];
                let pv = page_pv(&page_keys).await;
                let cmt = page_comment(&page_keys).await;
                let vote = {
                    let vote = self.artalk.vote_stats(key).await?;
                    let mut m = HashMap::new();
                    m.insert(vote.page_key.clone(), vote);
                    m
                };
                Ok(Some(FullIssue::new(i, &pv, &cmt, &vote, &mut m)))
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
        let user_ids = issues.iter().map(|i| i.user_id).collect_vec();
        let users = UserInfo::find_user_by_ids(&self.db, user_ids).await?;
        let mut id_u_map: HashMap<i32, user_info::Model> =
            users.into_iter().map(|u| (u.id, u)).collect();
        let pv = page_pv(&page_keys).await;
        let cmt = page_comment(&page_keys).await;
        let votes = self.artalk.batch_vote_stats(page_keys).await?;
        let votes: HashMap<String, VoteStats> =
            votes.into_iter().map(|v| (v.page_key.clone(), v)).collect();
        Ok(issues.map(|i| FullIssue::new(i, &pv, &cmt, &votes, &mut id_u_map)))
    }
}
