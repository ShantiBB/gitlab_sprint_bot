use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "GitLab Iteration Bot")]
pub struct Args {
    #[arg(long, required = true)]
    pub host: String,

    #[arg(long, required = true)]
    pub token: String,

    #[arg(long, required = true)]
    pub group_name: Option<String>,

    #[arg(long, num_args = 0.., value_delimiter = ' ')]
    pub assignees: Vec<String>,
}