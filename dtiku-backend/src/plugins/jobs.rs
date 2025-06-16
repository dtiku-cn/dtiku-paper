use dashmap::DashMap;
use derive_more::derive::Deref;
use dtiku_base::model::{enums::ScheduleTaskType, schedule_task::TaskInstance};
use spring::{
    app::AppBuilder,
    async_trait,
    plugin::{MutableComponentRegistry, Plugin},
};
use std::sync::Arc;

pub struct RunningJobsPlugin;

#[async_trait]
impl Plugin for RunningJobsPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        app.add_component(RunningJobs(Arc::new(TaskInstanceRegistry::default())));
    }
}

#[derive(Clone, Deref)]
pub struct RunningJobs(Arc<TaskInstanceRegistry>);

#[derive(Default, Deref)]
pub struct TaskInstanceRegistry(DashMap<ScheduleTaskType, TaskInstance>);

impl TaskInstanceRegistry {
    pub fn register_task_if_not_running(&self, task_type: ScheduleTaskType) -> TaskInstance {
        self.0
            .entry(task_type)
            .or_insert_with(Default::default)
            .clone()
    }

    #[inline]
    pub fn is_running(&self, task_type: ScheduleTaskType) -> bool {
        self.contains_key(&task_type)
    }
}
