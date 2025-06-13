use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use dashmap::DashMap;
pub(crate) use crate::models::iterations::Iteration;

pub async fn get_iterations(host: &str, token: &str, group_name: &str) -> Result<[Iteration; 2]> {
    let url = format!(
        "{}/api/v4/groups/{}/iterations?state=opened",
        host,
        group_name,
    );
    let client = Client::new();
    let iterations = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<Iteration>>()
        .await?;

    if iterations.is_empty() {
        println!("[ERROR] Итерации не найдены.")
    } else if iterations.len() < 2 {
        anyhow::bail!("[ERROR] Итераций меньше 2, всего: {}", iterations.len());
    }

    Ok([iterations[0].clone(), iterations[1].clone()])
}

pub struct IterationHandler<'a> {
    pub username: &'a str,
    pub developer_points: Arc<DashMap<String, (u32, u32)>>,
    pub has_release_or_customer_label: bool,
    pub has_to_review_or_to_test: bool,
}

impl<'a> IterationHandler<'a> {
    pub async fn move_reasons(
        &self,
        thresh_without: u32,
        thresh_all: u32,
    ) -> (bool, bool) {
        if self.has_release_or_customer_label {
            return (false, false);
        }

        if let Some(entry) = self.developer_points.get(self.username) {
            let (without, all) = *entry.value();
            let by_without = !self.has_to_review_or_to_test && without >= thresh_without;
            let by_all     = all >= thresh_all;
            (by_without, by_all)
        } else {
            (false, false)
        }
    }
}
