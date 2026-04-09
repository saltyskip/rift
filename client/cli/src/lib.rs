pub mod commands;
pub mod config;
pub mod context;
pub mod error;
pub mod ui;
pub mod util;

use clap::{Parser, Subcommand};
use clap_complete::Shell;

use error::CliError;

#[derive(Parser)]
#[command(name = "rift", version, about = "Deep links for humans and agents")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create or verify your Rift account and connect this machine
    Init {
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Connect this machine with an existing API key
    Login {
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Remove stored credentials from this machine
    Logout {
        #[arg(long)]
        json: bool,
    },
    /// Show the current account and verify the API key
    Whoami {
        #[arg(long)]
        json: bool,
    },
    /// Check what capabilities are unlocked and what to do next
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Manage deep links
    #[command(subcommand)]
    Links(LinksCommand),
    /// Register and manage mobile apps
    #[command(subcommand)]
    Apps(AppsCommand),
    /// Add and verify custom domains
    #[command(subcommand)]
    Domains(DomainsCommand),
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum LinksCommand {
    /// Create a new deep link
    Create {
        #[arg(long)]
        web_url: Option<String>,
        #[arg(long)]
        ios_deep_link: Option<String>,
        #[arg(long)]
        android_deep_link: Option<String>,
        #[arg(long)]
        ios_store_url: Option<String>,
        #[arg(long)]
        android_store_url: Option<String>,
        #[arg(long)]
        custom_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Preview how a link resolves across platforms
    Test {
        target: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum AppsCommand {
    /// Register iOS or Android app metadata
    Add {
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        bundle_id: Option<String>,
        #[arg(long)]
        team_id: Option<String>,
        #[arg(long)]
        package_name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        sha256_fingerprints: Option<Vec<String>>,
        #[arg(long)]
        app_name: Option<String>,
        #[arg(long)]
        icon_url: Option<String>,
        #[arg(long)]
        theme_color: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum DomainsCommand {
    /// Add and verify a custom domain with guided Cloudflare setup
    Setup {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init {
            email,
            base_url,
            json,
        } => commands::init::run(email, base_url, json).await,
        Command::Login { base_url, json } => commands::login::run(base_url, json).await,
        Command::Logout { json } => commands::logout::run(json),
        Command::Whoami { json } => commands::whoami::run(json).await,
        Command::Doctor { json } => commands::doctor::run(json).await,
        Command::Links(cmd) => match cmd {
            LinksCommand::Create {
                web_url,
                ios_deep_link,
                android_deep_link,
                ios_store_url,
                android_store_url,
                custom_id,
                json,
            } => {
                commands::create_link::run(commands::create_link::Args {
                    web_url,
                    ios_deep_link,
                    android_deep_link,
                    ios_store_url,
                    android_store_url,
                    custom_id,
                    json,
                })
                .await
            }
            LinksCommand::Test { target, json } => commands::test_link::run(target, json).await,
        },
        Command::Apps(cmd) => match cmd {
            AppsCommand::Add {
                platform,
                bundle_id,
                team_id,
                package_name,
                sha256_fingerprints,
                app_name,
                icon_url,
                theme_color,
                json,
            } => {
                commands::setup_app::run(commands::setup_app::Args {
                    platform,
                    bundle_id,
                    team_id,
                    package_name,
                    sha256_fingerprints,
                    app_name,
                    icon_url,
                    theme_color,
                    json,
                })
                .await
            }
        },
        Command::Domains(cmd) => match cmd {
            DomainsCommand::Setup {
                domain,
                provider,
                json,
            } => commands::setup_domain::run(domain, provider, json).await,
        },
        Command::Completions { shell } => commands::completions::run(shell),
    }
}
