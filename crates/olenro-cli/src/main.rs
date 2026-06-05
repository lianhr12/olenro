//! Olenro CLI
//!
//! Command-line interface for Olenro - Agent Workspace for AI Coding Tools

mod commands;
mod errors;
mod output;
mod state;

use clap::{Parser, Subcommand};
use state::CliState;

#[derive(Parser, Debug)]
#[command(
    name = "olenro",
    author = "Horace",
    version,
    about = "Olenro - Agent Workspace for AI Coding Tools",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Provider {
        #[command(subcommand)]
        subcommand: ProviderCommands,
    },
    Proxy {
        #[command(subcommand)]
        subcommand: ProxyCommands,
    },
    Mcp {
        #[command(subcommand)]
        subcommand: McpCommands,
    },
    Prompt {
        #[command(subcommand)]
        subcommand: PromptCommands,
    },
    Skill {
        #[command(subcommand)]
        subcommand: SkillCommands,
    },
    Session {
        #[command(subcommand)]
        subcommand: SessionCommands,
    },
    Usage {
        #[command(subcommand)]
        subcommand: UsageCommands,
    },
    GitSync {
        #[command(subcommand)]
        subcommand: GitSyncCommands,
    },
    Config {
        #[command(subcommand)]
        subcommand: ConfigCommands,
    },
    Doctor,
    Version,
}

#[derive(Subcommand, Debug)]
enum ProviderCommands {
    List {
        #[arg(long)]
        app: Option<String>,
    },
    Current {
        #[arg(long)]
        app: String,
    },
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        app: String,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        api_key: Option<String>,
    },
    Update {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        endpoint: Option<String>,
    },
    Delete {
        id: String,
    },
    Switch {
        id: String,
        #[arg(long)]
        app: String,
    },
}

#[derive(Subcommand, Debug)]
enum ProxyCommands {
    Start {
        #[arg(long, default_value = "15721")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        address: String,
    },
    Stop,
    Status,
    Takeover {
        app: String,
        #[arg(default_value = "on")]
        action: String,
    },
    Failover,
}

#[derive(Subcommand, Debug)]
enum McpCommands {
    List {
        #[arg(long)]
        app: Option<String>,
    },
    Add {
        name: String,
        #[arg(long)]
        command: String,
        #[arg(long)]
        args: Option<String>,
        #[arg(long)]
        app: Option<String>,
    },
    Delete {
        id: String,
    },
    Sync {
        app: String,
    },
    SyncAll,
}

#[derive(Subcommand, Debug)]
enum PromptCommands {
    List {
        #[arg(long)]
        app: Option<String>,
    },
    Set {
        content: String,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        app: Option<String>,
    },
    Get {
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        app: Option<String>,
    },
    Delete {
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum SkillCommands {
    List,
    Discover,
    Install {
        repo: String,
        #[arg(long)]
        apps: Option<String>,
    },
    Uninstall {
        id: String,
    },
    Update {
        id: String,
    },
    Sync {
        app: String,
    },
}

#[derive(Subcommand, Debug)]
enum SessionCommands {
    List {
        #[arg(long)]
        app: Option<String>,
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    Show {
        id: String,
        #[arg(long)]
        app: String,
    },
    Resume {
        id: String,
        #[arg(long)]
        app: String,
    },
}

#[derive(Subcommand, Debug)]
enum UsageCommands {
    Summary {
        #[arg(long, default_value = "30")]
        days: i32,
    },
    Trends {
        #[arg(long)]
        model: Option<String>,
    },
    Costs {
        #[arg(long)]
        provider: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum GitSyncCommands {
    Status,
    Push,
    Pull,
    Configure {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long, default_value = "main")]
        branch: String,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    Show {
        #[arg(long)]
        app: Option<String>,
    },
    Set {
        key: String,
        value: String,
        #[arg(long)]
        app: Option<String>,
    },
    Dir,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli_state = CliState::new();

    let cli = Cli::parse();

    match cli.command {
        Commands::Provider { subcommand } => match subcommand {
            ProviderCommands::List { app } => commands::provider::list(app, &cli_state).await?,
            ProviderCommands::Current { app } => commands::provider::current(&app, &cli_state).await?,
            ProviderCommands::Add { name, app, endpoint, api_key } => {
                commands::provider::add(&name, &app, &endpoint, api_key, &cli_state).await?
            }
            ProviderCommands::Update { id, name, endpoint } => {
                commands::provider::update(&id, name, endpoint, &cli_state).await?
            }
            ProviderCommands::Delete { id } => commands::provider::delete(&id, &cli_state).await?,
            ProviderCommands::Switch { id, app } => {
                commands::provider::switch(&id, &app, &cli_state).await?
            }
        },
        Commands::Proxy { subcommand } => match subcommand {
            ProxyCommands::Start { port, address } => commands::proxy::start(port, &address, &cli_state).await?,
            ProxyCommands::Stop => commands::proxy::stop(&cli_state).await?,
            ProxyCommands::Status => commands::proxy::status(&cli_state).await?,
            ProxyCommands::Takeover { app, action } => commands::proxy::takeover(&app, &action).await?,
            ProxyCommands::Failover => commands::proxy::failover(&cli_state).await?,
        },
        Commands::Mcp { subcommand } => match subcommand {
            McpCommands::List { app } => commands::mcp::list(app, &cli_state).await?,
            McpCommands::Add { name, command, args, app } => {
                commands::mcp::add(&name, &command, args, app, &cli_state).await?
            }
            McpCommands::Delete { id } => commands::mcp::delete(&id, &cli_state).await?,
            McpCommands::Sync { app } => commands::mcp::sync(&app, &cli_state).await?,
            McpCommands::SyncAll => commands::mcp::sync_all(&cli_state).await?,
        },
        Commands::Prompt { subcommand } => match subcommand {
            PromptCommands::List { app } => commands::prompt::list(app, &cli_state).await?,
            PromptCommands::Set { content, file, app } => {
                commands::prompt::set(&content, file, app, &cli_state).await?
            }
            PromptCommands::Get { file, app } => commands::prompt::get(file, app, &cli_state).await?,
            PromptCommands::Delete { id } => commands::prompt::delete(&id, &cli_state).await?,
        },
        Commands::Skill { subcommand } => match subcommand {
            SkillCommands::List => commands::skill::list(&cli_state).await?,
            SkillCommands::Discover => commands::skill::discover().await?,
            SkillCommands::Install { repo, apps } => commands::skill::install(&repo, apps, &cli_state).await?,
            SkillCommands::Uninstall { id } => commands::skill::uninstall(&id, &cli_state).await?,
            SkillCommands::Update { id } => commands::skill::update(&id, &cli_state).await?,
            SkillCommands::Sync { app } => commands::skill::sync(&app, &cli_state).await?,
        },
        Commands::Session { subcommand } => match subcommand {
            SessionCommands::List { app, limit } => commands::session::list(app, limit).await?,
            SessionCommands::Show { id, app } => commands::session::show(&id, &app).await?,
            SessionCommands::Resume { id, app } => commands::session::resume(&id, &app).await?,
        },
        Commands::Usage { subcommand } => match subcommand {
            UsageCommands::Summary { days } => commands::usage::summary(days).await?,
            UsageCommands::Trends { model } => commands::usage::trends(model).await?,
            UsageCommands::Costs { provider } => commands::usage::costs(provider).await?,
        },
        Commands::GitSync { subcommand } => match subcommand {
            GitSyncCommands::Status => commands::git_sync::status().await?,
            GitSyncCommands::Push => commands::git_sync::push().await?,
            GitSyncCommands::Pull => commands::git_sync::pull().await?,
            GitSyncCommands::Configure { repo, branch } => {
                commands::git_sync::configure(repo, &branch).await?
            }
        },
        Commands::Config { subcommand } => match subcommand {
            ConfigCommands::Show { app } => commands::config::handle(app).await?,
            ConfigCommands::Set { key, value, app } => {
                commands::config::set(&key, &value, app).await?
            }
            ConfigCommands::Dir => commands::config::dir().await?,
        },
        Commands::Doctor => commands::doctor::run().await?,
        Commands::Version => {
            println!("olenro {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
