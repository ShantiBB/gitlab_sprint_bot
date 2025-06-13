use serde::Deserialize;

use crate::models::groups::Group;

#[derive(Debug, Deserialize)]
pub struct GraphQLResponse {
    pub data: Data,
}

#[derive(Debug, Deserialize)]
pub struct Data {
    pub group: Group,
}
