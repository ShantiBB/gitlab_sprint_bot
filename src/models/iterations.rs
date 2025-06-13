use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Iteration {
    pub id: u64,
}
