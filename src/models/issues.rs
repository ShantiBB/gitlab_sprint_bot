use serde::Deserialize;

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
