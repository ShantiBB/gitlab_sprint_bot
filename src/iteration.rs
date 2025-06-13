use anyhow::Result;
use chrono::NaiveDate;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::sync::Arc;
use crate::logic::{Issue, Project};
use dashmap::DashMap;
const COUNT_SP_WITHOUT_LABELS: u32 = 15;
const COUNT_SP_ALL_ISSUES: u32 = 25;

#[derive(Deserialize, Debug, Clone)]
pub struct Iteration {
    pub id: u64,
    #[serde(rename = "start_date")]
    pub start_date: NaiveDate,
    #[serde(rename = "due_date")]
    pub due_date: NaiveDate,
}

pub fn get_iterations(host: &str, token: &str, group_name: &str) -> Result<[Iteration; 2]> {
    let url = format!(
        "https://{}/api/v4/groups/{}/iterations?state=opened",
        host,
        group_name,
    );
    let client = Client::new();
    let iterations = client
        .get(&url)
        .bearer_auth(token)
        .send()?
        .error_for_status()?
        .json::<Vec<Iteration>>()?;

    if iterations.is_empty() {
        println!("[ERROR] Итерации не найдены.")
    } else if iterations.len() < 2 {
        anyhow::bail!("[ERROR] Итераций меньше 2, всего: {}", iterations.len());
    }

    Ok([iterations[0].clone(), iterations[1].clone()])
}

pub fn update_issue_iteration(
    project_path: &str,
    issue_iid: u64,
    issue_url: &str,
    iteration_id: u64,
) -> Result<()> {
    // println!(
    //     "[INFO] Перенос задачи {} в итерацию {}",
    //     issue_url, iteration_id
    // );
    Ok(())
}

pub struct IterationHandler<'a> {
    pub username: &'a str,
    pub issue: &'a Issue,
    pub project: &'a Project,
    pub next_iteration: &'a Iteration,
    pub developer_points: Arc<DashMap<String, (u32, u32)>>,
    pub is_low_priority: bool,
    pub has_release_or_customer_label: bool,
    pub has_to_review_or_to_test: bool,
    pub weight: u32,
}

impl<'a> IterationHandler<'a> {
    pub fn should_move_by_labels(&self, sp_without_labels: u32) -> bool {
        !self.has_to_review_or_to_test && sp_without_labels >= COUNT_SP_WITHOUT_LABELS
    }

    pub fn should_move_by_total(&self, sp_all: u32) -> bool {
        sp_all >= COUNT_SP_ALL_ISSUES
    }

    pub fn update_iteration(&self, is_low_priority: bool, reason: &str) -> Result<(), String> {
        if is_low_priority {
            // println!("[INFO] Задача {} перемещается в следующую итерацию.", self.issue.web_url);
            update_issue_iteration(
                &self.project.web_url,
                self.issue.iid,
                &self.issue.web_url,
                self.next_iteration.id,
            ).map_err(|e| format!(
                "[ERROR] {}: ошибка при обновлении итерации ({}): {:?}",
                self.username, reason, e
            ))
        } else {
            Ok(())
        }
    }

    pub fn process(&self) {
        if self.has_release_or_customer_label {
            return;
        }

        if let Some(mut points) = self.developer_points.get_mut(self.username) {
            let (sp_without_labels, sp_all) = points.value_mut();
            if self.should_move_by_labels(*sp_without_labels) {
                if self.update_iteration(self.is_low_priority, "без лейблов").is_ok() {
                    *sp_without_labels = sp_without_labels.saturating_sub(self.weight);
                }
            }

            if self.should_move_by_total(*sp_all) {
                if self.update_iteration(self.is_low_priority, "все задачи").is_ok() {
                    *sp_all = sp_all.saturating_sub(self.weight);
                }
            }
        }
    }
}
