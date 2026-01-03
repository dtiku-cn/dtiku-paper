use anyhow::Context;
use dtiku_base::model::{
    schedule_task::{self, ActiveModel},
    ScheduleTask,
};
use dtiku_paper::model::{
    question, question_keypoint, ExamCategory, KeyPoint, Paper, PaperQuestion, Question,
};
use regex::Regex;
use sea_orm::{ActiveValue::Set, EntityTrait as _, QueryFilter};
use serde_json::Value;
use spring::{plugin::Service, tracing};
use spring_sea_orm::DbConn;
use std::collections::HashMap;

static REGEX_CONFIGS: &[(&str, &[&str])] = &[
    ("作文题/议论文", &["(议?论文)", "写一篇.*文章", "自拟题目.*写作?一?(?:篇|段)?(?:.*的)?文章"]),

    ("公文写作题/评论类", &["(时评|短评|社评)", "写一篇(评论)", "写点评", "反驳.*观点", "对.*(?:评析|评价|点评)"]),
    ("公文写作题/总结类", &["一份.*(总结|报告|综述)", "(调查报告|调研提纲|调研报告|考察报告|专题报告|简报|工作总结|汇报|导学材料)"]),
    ("公文写作题/宣传类", &["(公开信|导言|宣讲|编者按)", "一个?份?篇?.*(倡议|通知|通报|发言|讲话|讲解|宣传|介绍|推介|推荐|经验交流|主持词|新闻|报道)", "一封.*(信)"]),
    ("公文写作题/方案类", &["一份.*(方案|意见|建议|提案|备询要点)", "《.*(意见)》"]),

    ("综合类/词句解释类", &["(?:阐述|陈述|解释|分析|谈谈|谈一谈|谈一下|指出).*(看法|理解|见解|认识|含义)", "对.*(看法|理解|见解|认识|含义).*概括"]),
    ("综合类/概括主要内容类", &["(?:概括|提炼).*(看法|理解|见解|认识|含义)"]),

    ("公文写作题/其他类", &["一份.*"]),

    ("单一题/影响类", &["(变化|影响|作用|功能|意义|成效|危害|效果)"]),
    ("单一题/提出对策类", &["(做法|启示|对策|建议|措施|举措|经验|方式|途径)", "如何.*", "怎么.*", "解决(?:办法|方式)"]),
    ("单一题/原因类", &["(原因|理由|因素)", "为什么.*", "为何.*"]),
    ("单一题/问题类", &["(问题|困难|挑战|不足|劣势|难题)", "(现象|特征|特点|背景|现状)", "具?体?(表现)"]),

];

#[derive(Debug, Service)]
#[service(prototype)]
pub struct ShenlunCategorizeService {
    #[inject(component)]
    db: DbConn,
    #[inject(func = Self::build_regex_configs())]
    regex_configs: HashMap<(String, String), Vec<Regex>>,
    task: schedule_task::Model,
}

impl ShenlunCategorizeService {
    fn build_regex_configs() -> HashMap<(String, String), Vec<Regex>> {
        let mut configs = HashMap::new();
        for (category, patterns) in REGEX_CONFIGS {
            let category_type = category.split("/").nth(0).unwrap().to_string();
            let category_name = category.split("/").nth(1).unwrap().to_string();
            let regexes = patterns.iter().map(|p| Regex::new(p).unwrap()).collect();
            configs.insert((category_type, category_name), regexes);
        }
        configs
    }

    fn match_text(&self, html: &str) -> Option<(String, String)> {
        for ((ty, name), res) in &self.regex_configs {
            for re in res {
                if let Some(_caps) = re.captures(html) {
                    return Some((ty.to_string(), name.to_string()));
                }
            }
        }
        None
    }

    pub async fn start(&mut self) {
        let paper_type = ExamCategory::find_category_id_by_path(&self.db, "gwy/shenlun")
            .await
            .expect("gwy/shenlun category found failed")
            .expect("gwy/shenlun category id not found");

        self.stats_for_papers(paper_type).await.expect(&format!(
            "collect solution for papers for paper_type#{paper_type} failed"
        ));

        let _ = ScheduleTask::update(schedule_task::ActiveModel {
            id: Set(self.task.id),
            version: Set(self.task.version + 1),
            active: Set(false),
            ..Default::default()
        })
        .exec(&self.db)
        .await
        .is_err_and(|e| {
            tracing::error!("update task error: {:?}", e);
            false
        });
    }

    pub async fn stats_for_papers(&mut self, paper_type: i16) -> anyhow::Result<()> {
        let mut last_id = match &self.task.context {
            Value::Number(last_id) => last_id.as_i64().unwrap_or_default() as i32,
            _ => 0,
        };
        tracing::warn!("collect_for_papers({paper_type}) started");

        loop {
            let qids =
                PaperQuestion::find_by_paper_type_and_qid_gt(&self.db, paper_type, last_id).await?;
            if qids.is_empty() {
                tracing::warn!("collect_for_papers({paper_type}) finished");
                return Ok(());
            }
            let questions = Question::find_by_ids(&self.db, qids).await?;
            for q in questions {
                let qid = q.id;
                if let Err(e) = self.stats_shenlun_question(&q).await {
                    tracing::error!("collect_for_question({qid}) error: {e:?}");
                }
                last_id = qid.max(last_id);
                self.task = self.task.update_context(last_id, &self.db).await?;
            }
        }
    }

    async fn stats_shenlun_question(&self, q: &question::Model) -> anyhow::Result<()> {
        let html = q.content.replace(|c: char| c.is_whitespace(), "");

        let paper_type = q.paper_type;
        if let Some((ty, name)) = self.match_text(&html) {
            let kp = if name == "其他类" {
                let parent = KeyPoint::find_by_paper_type_and_name(&self.db, paper_type, &ty)
                    .await
                    .with_context(|| {
                        format!("KeyPoint::find_by_paper_type_and_name({paper_type},{ty})")
                    })?;
                if let Some(p) = parent {
                    KeyPoint::find_by_pid_and_name(&self.db, paper_type, p.id, &name)
                        .await
                        .with_context(|| {
                            format!("KeyPoint::find_by_pid_and_name({paper_type},{ty},{name})")
                        })?
                } else {
                    None
                }
            } else if name != "_" {
                KeyPoint::find_by_paper_type_and_name(&self.db, paper_type, &name)
                    .await
                    .with_context(|| {
                        format!("KeyPoint::find_by_paper_type_and_name({paper_type},{name})")
                    })?
            } else {
                KeyPoint::find_by_paper_type_and_name(&self.db, paper_type, &ty)
                    .await
                    .with_context(|| {
                        format!("KeyPoint::find_by_paper_type_and_name({paper_type},{ty})")
                    })?
            };

            if let Some(kp) = kp {
                let pqs = PaperQuestion::find_by_question_id(&self.db, q.id).await?;
                let year = if let Some(pq) = pqs.first() {
                    let p = Paper::find_by_id(pq.paper_id).one(&self.db).await?;
                    p.map(|p| p.year).unwrap_or(1970)
                } else {
                    1970
                };
                question_keypoint::ActiveModel {
                    question_id: Set(q.id),
                    key_point_id: Set(kp.id),
                    year: Set(year),
                    ..Default::default()
                }
                .insert_on_conflict(&self.db)
                .await
                .context("insert shenlun category question_keypoint failed")?;
            }
        }

        Ok(())
    }
}
