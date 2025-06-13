use std::sync::Arc;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use crate::utils::iteration::{Iteration, IterationHandler};
use dashmap::DashMap;
use reqwest::Client;

const COUNT_SP_WITHOUT_LABELS: u32 = 15;
const COUNT_SP_ALL_ISSUES: u32 = 25;

#[derive(Debug, Deserialize)]
pub struct GraphQLResponse {
    pub data: Data,
}

#[derive(Debug, Deserialize)]
pub struct Data {
    pub group: Group,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    pub projects: Projects,
}

#[derive(Debug, Deserialize)]
pub struct Projects {
    pub nodes: Vec<Project>,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(rename = "webUrl")]
    pub web_url: String,

    #[serde(rename = "issues")]
    pub issues: Issues,
}

#[derive(Debug, Deserialize)]
pub struct Issues {
    pub nodes: Vec<Issue>,
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub iid: String,

    pub weight: Option<u32>,

    pub labels: Labels,

    pub assignees: Assignees,

    #[serde(skip)]
    pub project_url: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Labels {
    pub nodes: Vec<LabelNode>,
}

#[derive(Debug, Deserialize)]
pub struct LabelNode {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Assignees {
    pub nodes: Vec<AssigneeNode>,
}

#[derive(Debug, Deserialize)]
pub struct AssigneeNode {
    pub username: String,
}

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
        let query = r#"
        query GetIterationIssues($group: ID!, $iterId: ID!) {
            group(fullPath: $group) {
                projects(first: 100, includeSubgroups: true) {
                    nodes {
                        webUrl
                        issues(state: opened, iterationId: [$iterId], first: 100) {
                            nodes {
                                iid
                                webUrl
                                weight
                                labels { nodes { title } }
                                assignees { nodes { username } }
                            }
                        }
                    }
                }
            }
        }
        "#;

        let variables = serde_json::json!({
            "group": self.group_name,
            "iterId": self.current_iteration.id,
        });

        let body = serde_json::json!({
            "query": query,
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

    pub async fn process(&mut self, issue: &Issue) -> Result<()> {
        let labels = &issue.labels;
        let is_low_priority = labels.nodes
            .iter()
            .any(|l| l.title == "priority::Minor" || l.title == "priority::Trivial");
        let has_release_or_customer_label = labels.nodes
            .iter()
            .any(
                |l| l.title.starts_with("release::")
                || l.title.starts_with("customer::")
            );
        let has_to_review_or_to_test = labels.nodes
            .iter()
            .any(|l| l.title == "status::to-review" || l.title == "status::to-test");
        let weight = issue.weight.unwrap_or(0);

        for assignee in &issue.assignees.nodes {
            let username = &assignee.username;

            if let Some(ref filter) = self.assignees_filter {
                if !filter.contains(username) {
                    continue;
                }
            }

            self.developer_points.entry(username.clone()).or_insert((0, 0));

            if !has_to_review_or_to_test {
                if let Some(mut entry) = self.developer_points.get_mut(username) {
                    entry.value_mut().0 = entry.value().0.saturating_add(weight);
                }
            }
            if let Some(mut entry) = self.developer_points.get_mut(username) {
                entry.value_mut().1 = entry.value().1.saturating_add(weight);
            }

            let handler = IterationHandler {
                username: &assignee.username,
                developer_points: Arc::clone(&self.developer_points),
                has_release_or_customer_label,
                has_to_review_or_to_test,
            };

            let (by_without, by_all) = handler
                .move_reasons(COUNT_SP_WITHOUT_LABELS, COUNT_SP_ALL_ISSUES)
                .await;

            if is_low_priority && (by_without || by_all) {
                let project_namespace = self.get_project_namespace(
                    &issue.project_url.clone().unwrap()
                ).await?;
                self.to_move.push((project_namespace, issue.iid.clone()));
                break;
            }
        }

        Ok(())
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

        println!("[INFO] Batch move response: {}", resp);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let issues_flat = self
            .get_group_issues()
            .await
            .map_err(|e| anyhow!("[ERROR] Ошибка при получении задач группы: {}", e))?;

        for issue in &issues_flat {
            self.process(issue).await?;
        }

        println!(
            "[INFO] Начался процесс перемещения: всего задач {}, переносятся: {}", 
            issues_flat.len(),
            self.to_move.len(),
        );
        self.batch_move_issues(self.to_move.clone())
            .await
            .map_err(|e| anyhow!("[ERROR] Ошибка при обновлении итерации: {}", e))?;
        Ok(())
    }
}
