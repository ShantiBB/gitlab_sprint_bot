use serde::Deserialize;

use crate::models::issues::Issues;

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