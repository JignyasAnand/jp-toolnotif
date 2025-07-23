use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PrIntent {
    total_commits: i32,
    groups_processed: i32,
    commits_indexed: i32,
    cost_burned: f64,
    status_log: Vec<String>,
}
