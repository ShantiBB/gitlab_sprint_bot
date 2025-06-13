use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use gitlab::api::{projects, Query};
use gitlab::api::issues::{GroupIssues, IssueState, IssueOrderBy, IssueIteration};
use gitlab::api::common::SortOrder;
use gitlab::Gitlab;
use serde::Deserialize;
use crate::iteration::{Iteration, IterationHandler};
use dashmap::DashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct Project {
    pub id: u64,
    pub web_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    pub username: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Issue {
    pub iid: u64,
    pub project_id: u64,
    pub web_url: String,
    pub assignees: Vec<User>,
    pub weight: Option<u32>,
    pub labels: Vec<String>,
}

pub struct BotState {
    pub client: Gitlab,
    pub group_name: String,
    pub current_iteration: Iteration,
    pub next_iteration: Iteration,
    pub assignees_filter: Option<Vec<String>>,
    pub developer_issues: HashMap<String, Vec<(Issue, Project)>>,
    pub developer_points: Arc<DashMap<String, (u32, u32)>>,
}

impl BotState {
    pub fn get_group_issues(&self) -> Result<Vec<Issue>> {
        let endpoint = GroupIssues::builder()
            .group(&self.group_name)
            .state(IssueState::Opened)
            .iteration(IssueIteration::Id(self.current_iteration.id))
            .order_by(IssueOrderBy::CreatedAt)
            .sort(SortOrder::Descending)
            .build()?;

        let issues: Vec<Issue> = endpoint
            .query(&self.client)
            .map_err(
                |e| anyhow::anyhow!(
                    "Ошибка запроса задач группы: {}", e
                )
            )?;
        Ok(issues)
    }

    pub fn get_project(&self, project_id: u64) -> Result<Project> {
        let endpoint = projects::Project::builder()
            .project(project_id)
            .build()?;
        let project: Project = endpoint
            .query(&self.client)
            .map_err(|e| anyhow::anyhow!("Ошибка запроса проекта: {}", e))?;

        Ok(project)
    }

    pub fn process_group(&mut self) -> Result<()> {
        let issues = self.get_group_issues()?;
        println!("[INFO] Группа ID={} — найдено задач: {}", &self.group_name, issues.len());

        for issue in issues.into_iter() {
            let project = self.get_project(issue.project_id)?;
            self.process_issue(&issue, &project)?
        }

        Ok(())
    }

    pub fn process_issue(&mut self, issue: &Issue, project: &Project) -> Result<()> {
        let labels = &issue.labels;
        let is_low_priority = labels
            .iter()
            .any(|l| l == "priority::Minor" || l == "priority::Trivial");
        let has_release_or_customer_label = labels
            .iter()
            .any(|l| l.starts_with("release::") || l.starts_with("customer::"));
        let has_to_review_or_to_test = labels
            .iter()
            .any(|l| l == "status::to-review" || l == "status::to-test");
        let weight = issue.weight.unwrap_or(0);

        for assignee in &issue.assignees {
            let username = &assignee.username;

            if let Some(ref filter) = self.assignees_filter {
                if !filter.contains(username) {
                    continue;
                }
            }

            self.developer_points.entry(username.clone()).or_insert((0, 0));
            if !has_to_review_or_to_test {
                if let Some(mut entry) = self.developer_points
                    .get_mut(username) {
                        entry.value_mut().0 = entry.value().0.saturating_add(weight);
                    }
            }
            if let Some(mut entry) = self.developer_points.get_mut(username) {
                entry.value_mut().1 = entry.value().1.saturating_add(weight);
            }

            let handler = IterationHandler {
                username,
                issue,
                project,
                next_iteration: &self.next_iteration,
                developer_points: Arc::clone(&self.developer_points),
                is_low_priority,
                has_release_or_customer_label,
                has_to_review_or_to_test,
                weight,
            };
            handler.process();
        }

        Ok(())
    }
    
}
