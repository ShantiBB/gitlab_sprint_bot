use serde::Deserialize;

use crate::models::projects::Projects;

#[derive(Debug, Deserialize)]
pub struct Group {
    pub projects: Projects,
}