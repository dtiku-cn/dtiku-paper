use anyhow::Context;
use chrono::{Duration, Local};
use dtiku_base::model::{user_info, UserInfo};
use dtiku_pay::model::OrderLevel;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use sea_orm::ActiveValue::Set;
use spring::plugin::service::Service;
use spring_sea_orm::DbConn;

#[derive(Debug, Clone, Service)]
pub struct UserService {
    #[inject(component)]
    db: DbConn,
}

impl UserService {
    pub async fn find_user_by_name(&self, name: &str) -> anyhow::Result<Option<user_info::Model>> {
        user_info::Entity::find()
            .filter(user_info::Column::Name.eq(name))
            .one(&self.db)
            .await
            .context("find user by name failed")
    }

    pub async fn get_user_detail(&self, user_id: i32) -> anyhow::Result<user_info::Model> {
        let u = UserInfo::find_user_by_id(&self.db, user_id)
            .await
            .context("get user detail failed")?;

        let mut u = match u {
            Some(u) => u,
            None => {
                return Err(anyhow::anyhow!("User not found: {}", user_id));
            }
        };

        // 检查用户的创建时间和过期时间是否均早于2025-11-07
        // 如果是，则将过期时间延长到当前时间 + 7天
        let cutoff_date = chrono::NaiveDate::from_ymd_opt(2025, 11, 7)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .expect("invalid cutoff date");
        
        if u.created < cutoff_date && u.expired < cutoff_date {
            let now = Local::now().naive_local();
            let new_expired = now + Duration::days(7);
            
            let updated_user = user_info::ActiveModel {
                id: Set(user_id),
                expired: Set(new_expired),
                ..Default::default()
            }
            .update(&self.db)
            .await
            .context("update user expired date failed")?;
            
            u = updated_user;
        }

        Ok(u)
    }

    #[allow(dead_code)]
    pub async fn confirm_user(
        &self,
        user_id: i32,
        order_level: OrderLevel,
    ) -> anyhow::Result<user_info::Model> {
        let now = Local::now().naive_local();
        let expires = now + Duration::days(order_level.days() as i64);
        user_info::ActiveModel {
            id: Set(user_id),
            expired: Set(expires),
            ..Default::default()
        }
        .update(&self.db)
        .await
        .with_context(|| format!("update user failed"))
    }
}

