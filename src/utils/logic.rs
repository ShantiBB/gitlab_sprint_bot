use std::sync::Arc;
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use reqwest::Client;

use crate::utils::iteration::Iteration;
use crate::models::graphql::GraphQLResponse;
use crate::models::issues::{AssigneeNode, Issue};
use crate::utils::constants::{
    COUNT_SP_WITHOUT_LABELS, 
    COUNT_SP_ALL_ISSUES,
    GET_ISSUES_QUERY
};

pub struct BotState {
    pub client: Client,
    pub token: String,
    pub host: String,
    pub group_name: String,
    pub current_iteration: Iteration,
    pub next_iteration: Iteration,
    pub assignees_filter: Option<Vec<String>>,
    pub developer_points: Arc<DashMap<String, (u32, u32)>>,
    pub to_move: Vec<(String, String)>
}

impl BotState {
    pub async fn get_group_issues(&mut self) -> Result<Vec<Issue>> {
        let variables = serde_json::json!({
            "group": self.group_name,
            "iterId": self.current_iteration.id,
        });

        let body = serde_json::json!({
            "query": GET_ISSUES_QUERY,
            "variables": variables,
        });

        let graphql_host = format!("{}/api/graphql", self.host);
        let response = self.client
            .post(graphql_host)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<GraphQLResponse>()
            .await?;

        let mut issues_flat = Vec::new();

        for project in response.data.group.projects.nodes {
            let proj_url = project.web_url.clone();

            for mut issue in project.issues.nodes {
                let titles = issue
                    .labels
                    .nodes
                    .iter()
                    .map(|l| &l.title).collect::<Vec<_>>();
                issue.has_low_priority_label = titles
                    .iter()
                    .any(|&t| t == "priority::Minor" || t == "priority::Trivial");
                issue.has_review_or_test_label = titles
                    .iter()
                    .any(|&t| t == "status::to-review" || t == "status::to-test");
                issue.has_release_or_customer_label = titles
                    .iter()
                    .any(
                        |&t| t.starts_with("release::")
                            || t.starts_with("customer::")
                    );
                issue.project_url = Some(proj_url.clone());
                issues_flat.push(issue);
            }
        }

        Ok(issues_flat)
    }
    
    pub async fn get_project_namespace(&self, project_url: &String) -> Result<String> {
        let normalized_host = self.host.trim_end_matches('/');
        let project_path = project_url
            .trim_end_matches('/')
            .strip_prefix(normalized_host)
            .unwrap_or_else(|| {
                panic!(
                    "[ERROR] Не удалось извлечь путь из project_url: {}",
                    project_url
                )
            })
            .trim_start_matches('/')
            .to_string();
        
        Ok(project_path)
    }

    fn check_assignees_flag(&self, username: &String) -> bool {
        if let Some(ref filter) = self.assignees_filter {
            if !filter.contains(username) {
                return true
            }
        }
        false
    }

    pub async fn move_reasons(&self, issue: &Issue, username: String) -> (bool, bool) {
        if issue.has_release_or_customer_label {
            return (false, false);
        }

        if let Some(entry) = self.developer_points.get(&username) {
            let (without, all) = *entry.value();
            let by_without = !issue
                .has_review_or_test_label && without >= COUNT_SP_WITHOUT_LABELS;
            let by_all     = all >= COUNT_SP_ALL_ISSUES;
            (by_without, by_all)
        } else {
            (false, false)
        }
    }

    pub async fn add_to_move_issues(&mut self, issue: &Issue) -> Result<()> {
        let assignees = &issue.assignees.nodes;
        let weight = issue.weight.unwrap_or(0);
        let mut should_move = false;
        
        for assignee in assignees {
            if self.check_assignees_flag(&assignee.username) {
                continue
            }
            let (by_without, by_all) = self.move_reasons(
                issue, assignee.username.clone()
            ).await;
            if issue.has_low_priority_label && (by_without || by_all) {
                should_move = true;
                break;
            }
        }

        if should_move {
            let project_namespace = self.get_project_namespace(
                issue.project_url.as_ref().unwrap(),
            ).await?;
            self.to_move.push((project_namespace, issue.iid.clone()));

            self.sub_weight_for_assignees(
                assignees,
                !issue.has_review_or_test_label,
                weight,
            );
        }

        Ok(())
    }

    fn add_weight_for_assignees(
        &self,
        assignees: &[AssigneeNode],
        without_review_to_test_labels: bool,
        weight: u32,
    ) {
        for assignee in assignees {
            let username = &assignee.username;
            if self.check_assignees_flag(username) {
                continue
            }
            
            self.developer_points.entry(username.clone()).or_insert((0, 0));
            if let Some(mut entry) = self.developer_points.get_mut(username) {
                if without_review_to_test_labels {
                    entry.value_mut().0 = entry.value().0.saturating_add(weight);
                }
                entry.value_mut().1 = entry.value().1.saturating_add(weight);
            }
        }
    }
    
    pub fn sub_weight_for_assignees(
        &self,
        assignees: &[AssigneeNode],
        by_without_labels: bool,
        weight: u32,
    ) {
        for assignee in assignees {
            let key = &assignee.username;
            if self.check_assignees_flag(key) {
                continue
            }
            
            if by_without_labels {
                if let Some(mut entry) = self.developer_points.get_mut(key) {
                    let new0 = entry.value().0.saturating_sub(weight);
                    entry.value_mut().0 = new0;
                }
            }
            if let Some(mut entry) = self.developer_points.get_mut(key) {
                let new1 = entry.value().1.saturating_sub(weight);
                entry.value_mut().1 = new1;
            }
        }
    }
    
    pub async fn batch_move_issues(
        &self,
        to_move: Vec<(String, String)>,
    ) -> Result<()> {
        if to_move.is_empty() {
            println!("[INFO] Нет задач для переноса.");
            return Ok(());
        }

        let mut fields = String::new();
        for (idx, (project_namespace, iid)) in to_move.iter().enumerate() {
            let graphql_iteration_id = format!(
                "\"gid://gitlab/Iteration/{}\"", 
                self.next_iteration.id
            );

            fields.push_str(&format!(
                r#"
                m{idx}: issueSetIteration(input: {{
                    projectPath: "{project_namespace}",
                    iid: "{iid}",
                    iterationId: {graphql_iteration_id}
                }}) {{
                    errors
                    issue {{ iid }}
                }}
                "#,
                idx = idx,
                project_namespace = project_namespace,
                iid = iid,
                graphql_iteration_id = graphql_iteration_id,
            ));
        }

        let mutation = format!("mutation BatchMoveIssues {{{}}}", fields);
        let graphql_host = format!("{}/api/graphql", &self.host);
        let body = serde_json::json!({ "query": mutation });
        let resp: serde_json::Value = self.client
            .post(graphql_host)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let ids: Vec<String> = resp["data"]
            .as_object()
            .unwrap()
            .values()
            .filter_map(|entry| {
                entry.get("issue")?.get("iid")?.as_str().map(|s| s.to_string())
            })
            .collect();
        println!("[INFO] Batch move response: ids [{}]", ids.join(", "));
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let issues_flat = self
            .get_group_issues()
            .await
            .map_err(|e| anyhow!("[ERROR] Ошибка при получении задач группы: {}", e))?;
        
        for issue in &issues_flat {
            let weight = issue.weight.unwrap_or(0);
            let assignees = &issue.assignees.nodes;
            let allows_without_review = !issue.has_review_or_test_label;
            self.add_weight_for_assignees(assignees, allows_without_review, weight);
        }
        for issue in &issues_flat {
            self.add_to_move_issues(issue).await?;
        }

        println!(
            "[INFO] Начался процесс перемещения: всего задач {}, переносятся: {}", 
            issues_flat.len(),
            self.to_move.len(),
        );
        self.batch_move_issues(self.to_move.clone())
            .await
            .map_err(|e| anyhow!("[ERROR] Ошибка при обновлении итерации: {}", e))?;
        
        for entry in self.developer_points.iter() {
            let name = entry.key();
            let (done, total) = *entry.value();
            println!("[INFO] {} - sp без лейблов: {} sp всех задач: {}", name, done, total);
        }
        Ok(())
    }
    
}
