use crate::plugins::fastembed::TxtEmbedding;
use crate::utils::regex::pick_year;
use anyhow::Context;
use dtiku_base::model::schedule_task;
use dtiku_base::model::schedule_task::Progress;
use dtiku_paper::model::exam_category;
use dtiku_paper::model::label;
use dtiku_paper::model::material;
use dtiku_paper::model::paper;
use dtiku_paper::model::paper::Chapters;
use dtiku_paper::model::paper::EssayCluster;
use dtiku_paper::model::paper::PaperBlock;
use dtiku_paper::model::paper::PaperChapter;
use dtiku_paper::model::paper_material;
use dtiku_paper::model::paper_question;
use dtiku_paper::model::question;
use dtiku_paper::model::solution;
use dtiku_paper::model::solution::AnswerAnalysis;
use dtiku_paper::model::solution::FillBlank;
use dtiku_paper::model::solution::MultiChoice;
use dtiku_paper::model::solution::SingleChoice;
use dtiku_paper::model::solution::StepAnalysis;
use dtiku_paper::model::solution::StepByStepAnswer;
use dtiku_paper::model::solution::TrueFalseChoice;
use dtiku_paper::model::Label;
use dtiku_paper::model::Question;
use futures::StreamExt;
use itertools::Itertools;
use scraper::Html;
use sea_orm::ActiveModelTrait;
use sea_orm::ConnectionTrait;
use sea_orm::Set;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use spring::plugin::service::Service;
use spring::tracing;
use spring_sea_orm::DbConn;
use spring_sqlx::sqlx;
use spring_sqlx::ConnectPool;
use sqlx::types::Json;
use sqlx::Row;
use std::collections::HashMap;

static SINGLE_CHOICE: [i16; 1] = [1];
static MULTI_CHOICE: [i16; 1] = [2];
static INDEFINITE_CHOICE: [i16; 1] = [3];
static BLANK_CHOICE: [i16; 2] = [4, 6];
static TRUE_FALSE: [i16; 1] = [5];
static FILL_BLANK: [i16; 2] = [61, 64];
static STEP_BY_STEP_ANSWER: [i16; 13] = [11, 12, 16, 21, 22, 23, 24, 25, 26, 101, 102, 301, 302];
static CLOSED_ENDED_ANSWER: [i16; 1] = [13];
static OPEN_ENDED_ANSWER: [i16; 3] = [14, 15, 303];

#[derive(Clone, Service)]
pub struct FenbiSyncService {
    source_db: ConnectPool,
    target_db: DbConn,
    txt_embedding: TxtEmbedding,
}

impl FenbiSyncService {
    pub async fn start(&self, task: &mut schedule_task::Model) -> anyhow::Result<()> {
        let mut progress = match task.context {
            Value::Null => {
                let total = self
                    .total("select count(*) as total from label where from_ty='fenbi'")
                    .await?;
                let progress = Progress {
                    name: "sync_label".to_string(),
                    total,
                    current: 0,
                };
                *task = task.update_progress(&progress, &self.target_db).await?;
                progress
            }
            _ => serde_json::from_value(task.context.clone())?,
        };
        if progress.name == "sync_label" {
            self.sync_label(task, &mut progress).await?;

            let total = self
                .total("select max(id) as total from paper where from_ty='fenbi'")
                .await?;
            progress = Progress {
                name: "sync_paper".to_string(),
                total,
                current: 0,
            };
            *task = task.update_progress(&progress, &self.target_db).await?;
        }
        if progress.name == "sync_paper" {
            self.sync_paper(task, &mut progress).await?;
        }
        Ok(())
    }

    async fn total(&self, sql: &str) -> anyhow::Result<i64> {
        Ok(sqlx::query(&sql)
            .fetch_one(&self.source_db)
            .await
            .with_context(|| format!("{sql} execute failed"))?
            .try_get("total")
            .context("get total failed")?)
    }

    async fn sync_label(
        &self,
        task: &mut schedule_task::Model,
        progress: &mut Progress<i64>,
    ) -> anyhow::Result<()> {
        let mut stream = sqlx::query_as::<_,OriginLabel>(r##"
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
        where from_ty = 'fenbi'
        order by exam_root,exam_name,paper_type,parent_label,label_name
        "##).fetch(&self.source_db);

        while let Some(row) = stream.next().await {
            match row {
                Ok(row) => {
                    let source_id = row.id;
                    let label = row.save_to(&self.target_db).await?;

                    sqlx::query("update label set target_id=? where id=?")
                        .bind(label.id)
                        .bind(source_id)
                        .execute(&self.source_db)
                        .await
                        .context("update source db label target_id failed")?;

                    if progress.increase(1) {
                        *task = task.update_progress(&progress, &self.target_db).await?;
                    }
                }
                Err(e) => tracing::error!("find label failed: {:?}", e),
            };
        }

        Ok(())
    }

    async fn sync_paper(
        &self,
        task: &mut schedule_task::Model,
        progress: &mut Progress<i64>,
    ) -> anyhow::Result<()> {
        while progress.current < progress.total {
            let current = progress.current;
            let next_step_id: i64 = current + 1000;
            let mut stream = sqlx::query_as::<_, OriginPaper>(
                r##"
                    select 
                        jsonb_extract_path_text(extra,'name') as name,
                        jsonb_extract_path_text(extra,'date') as date,
                        jsonb_extract_path_text(extra,'topic') as topic,
                        jsonb_extract_path_text(extra,'type') as ty,
                        jsonb_extract_path_text(extra,'chapters') as chapters,
                        id,
                        label_id
                    from paper
                    where from_ty = 'fenbi' and id > ? and id <= ?
                    "##,
            )
            .bind(current)
            .bind(next_step_id)
            .fetch(&self.source_db);

            while let Some(row) = stream.next().await {
                match row {
                    Ok(row) => {
                        let source_id = row.id;
                        let paper = self.save_paper(row).await?;

                        sqlx::query("update paper set target_id=? where id=?")
                            .bind(paper.id)
                            .bind(source_id)
                            .execute(&self.source_db)
                            .await
                            .context("update source db label target_id failed")?;

                        progress.current = source_id;
                        *task = task.update_progress(&progress, &self.target_db).await?;
                    }
                    Err(e) => tracing::error!("find label failed: {:?}", e),
                };
            }
        }
        Ok(())
    }

    async fn save_paper(&self, paper: OriginPaper) -> anyhow::Result<paper::Model> {
        let source_paper_id = paper.id;
        let target_label_id: i32 = sqlx::query("select target_id from label where id = ?")
            .bind(paper.label_id)
            .fetch_one(&self.source_db)
            .await
            .with_context(|| format!("find target_id for label#{}", paper.label_id))?
            .try_get("target_id")
            .context("get target_id failed")?;
        let paper = paper.save_to(&self.target_db, target_label_id).await?;

        self.sync_questions_and_materials(source_paper_id, &paper)
            .await?;

        Ok(paper)
    }

    async fn sync_questions_and_materials(
        &self,
        source_paper_id: i64,
        paper: &paper::Model,
    ) -> anyhow::Result<()> {
        let question_ids: Vec<QuestionIdNumber> = sqlx::query_as(
            r##"
            select
                question_id,
                number
            from paper_question
            where from_ty = 'fenbi'
            and paper_id = ?
            order by number
            "##,
        )
        .bind(source_paper_id)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find question_ids({source_paper_id}) failed"))?;

        let qids = question_ids.iter().map(|q| q.question_id).collect_vec();
        let qid_num_map: HashMap<_, _> = question_ids
            .into_iter()
            .map(|q| (q.question_id, q.number))
            .collect();

        let questions = sqlx::query_as::<_, OriginQuestion>(
            r##"
            select
                id,
                target_id,
                jsonb_extract_path(extra,'type') as ty,
                jsonb_extract_path(extra,'content') as content,
                jsonb_extract_path(extra,'accessories') as accessories,
                jsonb_extract_path(extra,'questionMeta','correctRatio') as correct_ratio,
                jsonb_extract_path(extra,'correctAnswer') as correct_answer,
                jsonb_extract_path(extra,'solution') as solution,
                jsonb_extract_path(extra,'solutionAccessories') as solution_accessories,
                jsonb_extract_path(extra,'material') as material,
                jsonb_extract_path(extra,'keypoints') as keypoints
            from question
            where from_ty = 'fenbi'
            and id in (?)
        "##,
        )
        .bind(qids)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find question failed"))?;

        let material_ids: Vec<MaterialIdNumber> = sqlx::query_as(
            r##"
            select
                material_id,
                number
            from paper_material
            where from_ty = 'fenbi'
            and paper_id = ?
            order by number
            "##,
        )
        .bind(source_paper_id)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find material_ids({source_paper_id}) failed"))?;

        let mids = material_ids.iter().map(|m| m.material_id).collect_vec();
        let mid_num_map: HashMap<_, _> = material_ids
            .into_iter()
            .map(|m| (m.material_id, m.number))
            .collect();

        let materials = sqlx::query_as::<_, OriginMaterial>(
            r##"
            select
                id,
                target_id,
                jsonb_extract_path(extra,'type') as ty,
                jsonb_extract_path(extra,'content') as content,
                jsonb_extract_path(extra,'accessories') as accessories
            from material
            where from_ty = 'fenbi'
            and id in (?)
        "##,
        )
        .bind(mids)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find material failed"))?;

        self.save_questions_and_materials(questions, materials, paper, &qid_num_map, &mid_num_map)
            .await?;
        todo!()
    }

    async fn save_questions_and_materials(
        &self,
        questions: Vec<OriginQuestion>,
        materials: Vec<OriginMaterial>,
        paper: &paper::Model,
        qid_num_map: &HashMap<i64, i32>,
        mid_num_map: &HashMap<i64, i32>,
    ) -> anyhow::Result<()> {
        for q in questions {
            let correct_ratio = q.correct_ratio.expect("correct_ratio is none");
            let num = qid_num_map
                .get(&q.id)
                .expect("qid is not exists in qid_num_map");
            let mut question = q.to_question(&self.txt_embedding)?;
            question.exam_id = Set(paper.exam_id);
            question.paper_type = Set(paper.paper_type);
            let q_in_db = question
                .insert(&self.target_db)
                .await
                .context("insert question failed")?;
            let mut solution = q.to_solution()?;
            solution.question_id = Set(q_in_db.id);
            solution
                .insert(&self.target_db)
                .await
                .context("insert solution failed")?;

            paper_question::ActiveModel {
                paper_id: Set(paper.id),
                question_id: Set(q_in_db.id),
                sort: Set(num.clone() as i16),
                // category: Set(), TODO
                correct_ratio: Set(correct_ratio),
                ..Default::default()
            }
            .insert(&self.target_db)
            .await
            .context("insert paper_question failed")?;
        }

        for m in materials {
            let num = mid_num_map
                .get(&m.id)
                .expect("mid is not exists in qid_num_map");
            let material = TryInto::<material::ActiveModel>::try_into(m)?;

            let m_in_db = material
                .insert(&self.target_db)
                .await
                .context("insert paper_material failed")?;

            paper_material::ActiveModel {
                paper_id: Set(paper.id),
                material_id: Set(m_in_db.id),
                sort: Set(num.clone() as i16),
            }
            .insert(&self.target_db)
            .await
            .context("insert paper_material failed")?;
        }

        todo!()
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OriginLabel {
    exam_root: Option<String>,
    exam_root_prefix: Option<String>,
    exam_name: Option<String>,
    exam_prefix: Option<String>,
    paper_type: Option<String>,
    paper_prefix: Option<String>,
    parent_label: Option<String>,
    label_name: Option<String>,
    id: i64,
}

impl OriginLabel {
    async fn save_to<C: ConnectionTrait>(self, db: &C) -> anyhow::Result<label::Model> {
        let root = exam_category::ActiveModel {
            pid: Set(0),
            name: Set(self.exam_root.expect("exam_root is none")),
            prefix: Set(self.exam_root_prefix.expect("exam_root_prefix is none")),
            ..Default::default()
        }
        .insert_on_conflict(db)
        .await
        .context("insert root exam_category failed")?;

        let second = exam_category::ActiveModel {
            pid: Set(root.id),
            name: Set(self.exam_name.expect("exam_name is none")),
            prefix: Set(self.exam_prefix.expect("exam_prefix is none")),
            ..Default::default()
        }
        .insert_on_conflict(db)
        .await
        .context("insert second exam_category failed")?;

        let leaf = exam_category::ActiveModel {
            pid: Set(second.id),
            name: Set(self.paper_type.expect("paper_type is none")),
            prefix: Set(self.paper_prefix.expect("paper_prefix is none")),
            ..Default::default()
        }
        .insert_on_conflict(db)
        .await
        .context("insert leaf exam_category failed")?;

        let label_name = self.label_name.expect("label_name is none");
        let label = match self.parent_label {
            None => label::ActiveModel {
                name: Set(label_name),
                pid: Set(0),
                exam_id: Set(root.id),
                paper_type: Set(leaf.id),
                ..Default::default()
            },
            Some(parent) => {
                let parent_label =
                    Label::find_by_exam_id_and_paper_type_and_name(db, root.id, leaf.id, &parent)
                        .await?;
                let parent_label = match parent_label {
                    None => label::ActiveModel {
                        name: Set(parent),
                        pid: Set(0),
                        exam_id: Set(root.id),
                        paper_type: Set(leaf.id),
                        ..Default::default()
                    }
                    .insert_on_conflict(db)
                    .await
                    .context("insert parent label failed")?,
                    Some(parent_label) => parent_label,
                };
                label::ActiveModel {
                    name: Set(label_name),
                    pid: Set(parent_label.id),
                    exam_id: Set(root.id),
                    paper_type: Set(leaf.id),
                    ..Default::default()
                }
            }
        };

        let label = label
            .insert_on_conflict(db)
            .await
            .context("insert label failed")?;
        Ok(label)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OriginPaper {
    name: Option<String>,
    date: Option<String>,
    topic: Option<String>,
    ty: Option<i64>,
    chapters: Option<String>,
    id: i64,
    label_id: i64,
}

impl OriginPaper {
    async fn save_to<C: ConnectionTrait>(
        self,
        db: &C,
        label_id: i32,
    ) -> anyhow::Result<paper::Model> {
        let label = Label::find_by_id_with_cache(db, label_id)
            .await
            .with_context(|| format!("Label::find_by_id_with_cache({label_id}) failed"))?
            .expect(&format!("label#{label_id} not exists"));

        let year = pick_year(&self.date.expect("date is none")).expect("year not found");
        let mut active_model = paper::ActiveModel {
            title: Set(self.name.expect("name is none")),
            year: Set(year),
            exam_id: Set(label.exam_id),
            paper_type: Set(label.paper_type),
            label_id: Set(label.id),
            ..Default::default()
        };

        let chapters = &self.chapters.expect("chapters is none");
        let chapters: Vec<OriginChapter> =
            serde_json::from_str(chapters).context("parse chapters failed")?;

        let extra_value = if self.ty.expect("type is none") == 0 {
            let cs = Chapters {
                desc: None,
                chapters: chapters.into_iter().map(|m| m.into()).collect(),
            };
            serde_json::to_value(cs).context("Chapters to_value failed")?
        } else {
            let ec = EssayCluster {
                topic: self.topic,
                blocks: chapters.into_iter().map(|m| m.into()).collect(),
            };
            serde_json::to_value(ec).context("EssayCluster to_value failed")?
        };
        active_model.extra = Set(extra_value);

        let paper = active_model
            .insert_on_conflict(db)
            .await
            .context("insert paper failed")?;

        Ok(paper)
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginChapter {
    pub desc: String,
    pub name: String,
    pub question_count: i64,
}

impl Into<PaperChapter> for OriginChapter {
    fn into(self) -> PaperChapter {
        PaperChapter {
            desc: self.desc,
            name: self.name,
            count: self.question_count as i16,
        }
    }
}

impl Into<PaperBlock> for OriginChapter {
    fn into(self) -> PaperBlock {
        PaperBlock {
            desc: self.desc,
            name: self.name,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct QuestionIdNumber {
    question_id: i64,
    number: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct OriginQuestion {
    id: i64,
    target_id: Option<i32>,
    ty: i16,
    content: String,
    accessories: Json<Vec<QuestionAccessory>>,
    material: Json<OriginMaterial>,
    keypoints: Json<Vec<OriginKeyPoint>>,
    correct_ratio: Option<f32>,
    correct_answer: Option<String>,
    solution: Option<String>,
    solution_accessories: Json<Vec<SolutionAccessory>>,
}

impl OriginQuestion {
    fn to_question(&self, model: &TxtEmbedding) -> anyhow::Result<question::ActiveModel> {
        let Self { ty, content, .. } = self;
        let mut options_string = String::new();
        let extra = if SINGLE_CHOICE.contains(ty) {
            let list = self.filter_accessory(|a| [101i16, 102i16].contains(&a.ty));
            let os = list
                .last()
                .expect("SingleChoice don't contains 101/102 options");
            let options = os
                .options
                .clone()
                .expect("SingleChoice 101/102 options is none");
            options_string = options.iter().join("\n");
            question::QuestionExtra::SingleChoice(options)
        } else if MULTI_CHOICE.contains(ty) {
            let list = self.filter_accessory(|a| [101i16, 102i16].contains(&a.ty));
            let os = list
                .last()
                .expect("MultiChoice don't contains 101/102 options");
            let options = os
                .options
                .clone()
                .expect("MultiChoice 101/102 options is none");
            options_string = options.iter().join("\n");
            question::QuestionExtra::MultiChoice(options)
        } else if INDEFINITE_CHOICE.contains(ty) {
            let list = self.filter_accessory(|a| [101i16, 102i16].contains(&a.ty));
            let os = list
                .last()
                .expect("IndefiniteChoice don't contains 101/102 options");
            let options = os
                .options
                .clone()
                .expect("IndefiniteChoice 101/102 options is none");
            options_string = options.iter().join("\n");
            question::QuestionExtra::IndefiniteChoice(options)
        } else if BLANK_CHOICE.contains(ty) {
            let list = self.filter_accessory(|a| [101i16, 102i16].contains(&a.ty));
            let os = list
                .last()
                .expect("BlankChoice don't contains 101/102 options");
            let options = os
                .options
                .clone()
                .expect("BlankChoice 101/102 options is none");
            options_string = options.iter().join("\n");
            question::QuestionExtra::BlankChoice(options)
        } else if TRUE_FALSE.contains(ty) {
            question::QuestionExtra::TrueFalse
        } else if STEP_BY_STEP_ANSWER.contains(ty) {
            let list = self.filter_accessory(|a| [182i16].contains(&a.ty));
            let os = list.last().expect("BlankChoice don't contains 182 options");
            question::QuestionExtra::StepByStepQA(question::QA {
                title: os.title.clone().expect("StepByStepQA title is none"),
                word_count: os.word_count,
                material_ids: os.material_indexes.clone(),
            })
        } else if CLOSED_ENDED_ANSWER.contains(ty) {
            let list = self.filter_accessory(|a| [182i16].contains(&a.ty));
            let os = list
                .last()
                .expect("ClosedEndedQA don't contains 182 options");
            question::QuestionExtra::ClosedEndedQA(question::QA {
                title: os.title.clone().expect("StepByStepQA title is none"),
                word_count: os.word_count,
                material_ids: os.material_indexes.clone(),
            })
        } else if OPEN_ENDED_ANSWER.contains(ty) {
            let list = self.filter_accessory(|a| [182i16].contains(&a.ty));
            let os = list.last().expect("BlankChoice don't contains 182 options");
            question::QuestionExtra::ClosedEndedQA(question::QA {
                title: os.title.clone().expect("StepByStepQA title is none"),
                word_count: os.word_count,
                material_ids: os.material_indexes.clone(),
            })
        } else if FILL_BLANK.contains(ty) {
            let vec = self.filter_accessory(|a| [182i16].contains(&a.ty));
            let os = vec.last().expect("BlankChoice don't contains 182 options");
            question::QuestionExtra::ClosedEndedQA(question::QA {
                title: os.title.clone().expect("StepByStepQA title is none"),
                word_count: os.word_count,
                material_ids: os.material_indexes.clone(),
            })
        } else {
            Err(anyhow::anyhow!("ty#{ty} is not defined"))?
        };
        let html = Html::parse_fragment(&format!("{content}\n{options_string}"));
        let txt: String = html.root_element().text().collect();
        let mut embedding = model.embed(vec![txt], None)?;
        let embedding = embedding.remove(0);
        Ok(question::ActiveModel {
            content: Set(content.into()),
            extra: Set(serde_json::to_value(extra)?),
            embedding: Set(embedding),
            ..Default::default()
        })
    }

    fn to_solution(&self) -> anyhow::Result<solution::ActiveModel> {
        let Self {
            ty,
            correct_answer,
            solution,
            solution_accessories,
            ..
        } = self;
        let extra = if SINGLE_CHOICE.contains(ty) {
            solution::SolutionExtra::SingleChoice(SingleChoice {
                answer: correct_answer
                    .clone()
                    .expect("correct_answer is none")
                    .parse()
                    .expect("correct_answer parse failed"),
                analysis: solution.clone().expect("solution is none"),
            })
        } else if MULTI_CHOICE.contains(ty) {
            let answer: Result<Vec<u16>, std::num::ParseIntError> = correct_answer
                .clone()
                .expect("correct_answer is none")
                .split(",")
                .map(|a| a.parse())
                .collect();
            solution::SolutionExtra::MultiChoice(MultiChoice {
                answer: answer?,
                analysis: solution.clone().expect("solution is none"),
            })
        } else if INDEFINITE_CHOICE.contains(ty) {
            let answer: Result<Vec<u16>, std::num::ParseIntError> = correct_answer
                .clone()
                .expect("correct_answer is none")
                .split(",")
                .map(|a| a.parse())
                .collect();
            solution::SolutionExtra::IndefiniteChoice(MultiChoice {
                answer: answer?,
                analysis: solution.clone().expect("solution is none"),
            })
        } else if BLANK_CHOICE.contains(ty) {
            solution::SolutionExtra::BlankChoice(SingleChoice {
                answer: correct_answer
                    .clone()
                    .expect("correct_answer is none")
                    .parse()
                    .expect("correct_answer parse failed"),
                analysis: solution.clone().expect("solution is none"),
            })
        } else if TRUE_FALSE.contains(ty) {
            solution::SolutionExtra::TrueFalse(TrueFalseChoice {
                answer: correct_answer
                    .clone()
                    .expect("correct_answer is none")
                    .parse()
                    .expect("correct_answer parse failed"),
                analysis: solution.clone().expect("solution is none"),
            })
        } else if FILL_BLANK.contains(ty) {
            let blanks = vec![];// TOOD:
            solution::SolutionExtra::FillBlank(FillBlank {
                blanks: blanks,
                analysis: solution.clone().expect("solution is none"),
            })
        } else {
            if solution_accessories.len() < 1 {
                solution::SolutionExtra::ClosedEndedQA(AnswerAnalysis {
                    analysis: solution.clone().expect("solution is none"),
                    answer: correct_answer.clone().expect("correct_answer is none"),
                })
            } else if solution_accessories.len() > 1 && correct_answer.is_none() {
                let analysis = solution_accessories
                    .0
                    .into_iter()
                    .map(|a| a.into())
                    .collect();
                solution::SolutionExtra::OpenEndedQA(StepByStepAnswer { analysis: analysis })
            } else {
                let analysis = solution_accessories
                    .0
                    .into_iter()
                    .map(|a| a.into())
                    .collect();
                solution::SolutionExtra::OpenEndedQA(StepByStepAnswer { analysis: analysis })
            }
        } 
        Ok(solution::ActiveModel {
            extra: Set(serde_json::to_value(extra)?),
            ..Default::default()
        })
    }

    fn filter_accessory<F>(&self, filter: F) -> Vec<&QuestionAccessory>
    where
        F: Fn(&QuestionAccessory) -> bool,
    {
        self.accessories
            .0
            .iter()
            .filter(|a| filter(a))
            .collect_vec()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuestionAccessory {
    #[serde(rename = "type")]
    pub ty: i16,
    pub options: Option<Vec<String>>,
    pub name: Option<String>,
    pub label: Option<String>,
    pub content: Option<String>,
    pub is_member_control: Option<i64>,
    pub score: Option<f64>,
    pub title: Option<String>,
    pub blank_type: Option<i64>,
    pub word_count: Option<i16>,
    #[serde(default)]
    pub material_indexes: Vec<i32>,
    pub url: Option<String>,
    pub audio_id: Option<String>,
    pub duration: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SolutionAccessory {}

#[derive(Debug, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct OriginMaterial {
    pub id: i64,
    pub target_id: Option<i32>,
    pub content: String,
    pub accessories: Json<Vec<MaterialAccessory>>,
}

impl TryInto<material::ActiveModel> for OriginMaterial {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<material::ActiveModel, Self::Error> {
        let extra = self
            .accessories
            .0
            .into_iter()
            .map(|a| a.try_into())
            .collect::<anyhow::Result<Vec<material::MaterialExtra>>>()?;
        let mut am = material::ActiveModel {
            content: Set(self.content),
            extra: Set(serde_json::to_value(extra).context("serde encode failed")?),
            ..Default::default()
        };
        if let Some(id) = self.target_id {
            am.id = Set(id);
        }
        Ok(am)
    }
}

impl TryInto<material::MaterialExtra> for MaterialAccessory {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<material::MaterialExtra, Self::Error> {
        match self.ty {
            151 => Ok(material::MaterialExtra::Translation(
                self.translation.expect("translation is none"),
            )),
            181 => match self.label.expect("label is none").as_str() {
                "materialExplain" => Ok(material::MaterialExtra::MaterialExplain(
                    self.content.expect("materialExplain content is none"),
                )),
                "transcript" => Ok(material::MaterialExtra::Transcript(
                    self.content.expect("transcript content is none"),
                )),
                "zdch" => Ok(material::MaterialExtra::Dictionary(
                    self.content.expect("zdch content is none"),
                )),
                _unknown => Err(anyhow::anyhow!("unknown material label:{_unknown}")),
            },
            185 => Ok(material::MaterialExtra::Audio(
                self.url.expect("Audio url is none"),
            )),
            _unknown => Err(anyhow::anyhow!(
                "unknown material accessory type:{_unknown}"
            )),
        }
    }
}

impl Into<StepAnalysis> for SolutionAccessory {
    fn into(self) -> StepAnalysis {
        todo!()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialAccessory {
    #[serde(rename = "type")]
    pub ty: i64,
    pub label: Option<String>,
    pub content: Option<String>,
    pub translation: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OriginKeyPoint {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct MaterialIdNumber {
    material_id: i64,
    number: i32,
}
