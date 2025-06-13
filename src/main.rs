mod args;
mod iteration;
mod logic;

use std::collections::HashMap;
use std::sync::Arc;
use args::Args;
use iteration::get_iterations;
use clap::Parser;
use dashmap::DashMap;
use gitlab::Gitlab;
use crate::logic::BotState;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let gl = Gitlab::new(&args.host, args.token.clone())
        .map_err(|e| anyhow::anyhow!("Не удалось создать клиент GitLab: {}", e))?;
    let group_name = args
        .group_name.as_ref()
        .ok_or_else(|| { anyhow::anyhow!("Не указан аргумент --group-name.") })?;
    
    let [current, next] = get_iterations(&args.host, &args.token, group_name)?;
    for (i, iter) in [current.clone(), next.clone()].iter().enumerate() {
        let iter_type = if i == 0 { "Текущая" } else { "Следующая" };
        println!(
            "[INFO] {} итерация: id={}, start_date={}, due_date={}",
            iter_type,
            iter.id,
            iter.start_date,
            iter.due_date,
        );
    }

    let mut state = BotState {
        client: gl,
        group_name: group_name.to_string(),
        current_iteration: current,
        next_iteration: next,
        assignees_filter: if args.assignees.is_empty() { None } 
            else { Some(args.assignees.clone()) },
        developer_issues: HashMap::new(),
        developer_points: Arc::new(DashMap::new()),
    };
    
    state.process_group()?;

    Ok(())
}
