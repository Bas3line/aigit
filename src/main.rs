use clap::{Parser, Subcommand};
use aigit::commands;

#[derive(Parser)]
#[command(name = "aigit")]
#[command(about = "AI-powered version control system")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(long)]
        bare: bool,
    },
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    Add {
        files: Vec<String>,
        #[arg(short, long)]
        all: bool,
    },
    Commit {
        #[arg(short, long)]
        message: Option<String>,
        #[arg(long)]
        amend: bool,
        #[arg(long)]
        ai_review: bool,
        #[arg(short, long)]
        signoff: bool,
    },
    Status {
        #[arg(short, long)]
        porcelain: bool,
    },
    Log {
        #[arg(short, long)]
        oneline: bool,
        #[arg(short, long)]
        graph: bool,
        #[arg(long)]
        ai_summary: bool,
    },
    Branch {
        name: Option<String>,
        #[arg(short, long)]
        delete: Option<String>,
        #[arg(long)]
        ai_suggest: bool,
    },
    Checkout {
        target: String,
        #[arg(short, long)]
        create: bool,
    },
    Diff {
        #[arg(long)]
        cached: bool,
        #[arg(long)]
        ai_explain: bool,
    },
    Merge {
        branch: String,
        #[arg(long)]
        ai_resolve: bool,
    },
    Review {
        #[arg(long)]
        full: bool,
    },
    Suggest {
        #[command(subcommand)]
        action: SuggestCommands,
    },
    Push {
        branch: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    Set {
        key: String,
        value: String,
    },
    Get {
        key: String,
    },
    List,
    User {
        name: Option<String>,
        #[arg(long)]
        email: Option<String>,
    },
}

#[derive(Subcommand)]
enum SuggestCommands {
    Commit,
    Branch,
    Refactor,
    Tests,
    Cleanup,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { bare } => commands::init::run(*bare).await?,
        Commands::Config { action } => {
            let config_action = match action {
                ConfigAction::Set { key, value } => commands::config::ConfigAction::Set {
                    key: key.clone(),
                    value: value.clone(),
                },
                ConfigAction::Get { key } => commands::config::ConfigAction::Get {
                    key: key.clone(),
                },
                ConfigAction::List => commands::config::ConfigAction::List,
                ConfigAction::User { name, email } => commands::config::ConfigAction::User {
                    name: name.clone(),
                    email: email.clone(),
                },
            };
            commands::config::run(&config_action).await?
        },
        Commands::Add { files, all } => commands::add::run(files.clone(), *all).await?,
        Commands::Commit { message, amend, ai_review, signoff } => {
            commands::commit::run(message.clone(), *amend, *ai_review, *signoff).await?
        },
        Commands::Status { porcelain } => commands::status::run(*porcelain).await?,
        Commands::Log { oneline, graph, ai_summary } => {
            commands::log::run(*oneline, *graph, *ai_summary).await?
        },
        Commands::Branch { name, delete, ai_suggest } => {
            commands::branch::run(name.clone(), delete.clone(), *ai_suggest).await?
        },
        Commands::Checkout { target, create } => {
            commands::checkout::run(target.clone(), *create).await?
        },
        Commands::Diff { cached, ai_explain } => {
            commands::diff::run(*cached, *ai_explain).await?
        },
        Commands::Merge { branch, ai_resolve } => {
            commands::merge::run(branch.clone(), *ai_resolve).await?
        },
        Commands::Review { full } => commands::review::run(*full).await?,
        Commands::Suggest { action } => {
            match action {
                SuggestCommands::Commit => commands::suggest::commit().await?,
                SuggestCommands::Branch => commands::suggest::branch().await?,
                SuggestCommands::Refactor => commands::suggest::refactor().await?,
                SuggestCommands::Tests => commands::suggest::tests().await?,
                SuggestCommands::Cleanup => commands::suggest::cleanup().await?,
            }
        },
        Commands::Push { branch } => {
            commands::push::run(branch.clone()).await?
        },
    }

    Ok(())
}
