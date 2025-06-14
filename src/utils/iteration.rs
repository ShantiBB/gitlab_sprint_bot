use anyhow::Result;
use reqwest::Client;

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
