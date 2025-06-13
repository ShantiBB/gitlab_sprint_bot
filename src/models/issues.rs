use serde::Deserialize;

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
