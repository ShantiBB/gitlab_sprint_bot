mod utils;
mod models;

use std::sync::Arc;
use std::time::Instant;
use anyhow::anyhow;
use clap::Parser;
use dashmap::DashMap;
use reqwest::Client;

use utils::args::Args;
use utils::iteration::get_iterations;
use crate::utils::logic::BotState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start_time = Instant::now();
    
    let args = Args::parse();
    let group_name = args
        .group_name.as_ref()
        .ok_or_else(|| { anyhow::anyhow!("Не указан аргумент --group-name.") })?;
    
    let [current, next] = get_iterations(
        &args.host, 
        &args.token, 
        group_name
    ).await?;
    

    let client = Client::new();
    let mut state = BotState {
        client,
        token: args.token.to_string(),
        host: args.host.to_string(),
        group_name: group_name.to_string(),
        current_iteration: current,
        next_iteration: next,
        assignees_filter: if args.assignees.is_empty() { None } 
            else { Some(args.assignees.clone()) },
        developer_points: Arc::new(DashMap::new()),
        to_move: vec![],
    };
    
    state.run().await.map_err(|e| anyhow!("[ERROR] {}", e))?;

    let duration = start_time.elapsed();
    println!(
        "[INFO] Скрипт выполнен за {:.2?} секунд",
        duration
    );
    
    Ok(())
}
