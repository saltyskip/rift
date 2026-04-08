mod commands;
mod config;
mod ui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rift", about = "Relay onboarding-first CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        json: bool,
    },
    CreateLink {
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
    TestLink {
        target: String,
        #[arg(long)]
        json: bool,
    },
    Doctor {
        #[arg(long)]
        json: bool,
    },
    Setup {
        #[command(subcommand)]
        command: SetupCommand,
    },
}

#[derive(Subcommand)]
enum SetupCommand {
    App {
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
    Domain {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init {
            email,
            base_url,
            json,
        } => commands::init::run(email, base_url, json).await?,
        Command::CreateLink {
            web_url,
            ios_deep_link,
            android_deep_link,
            ios_store_url,
            android_store_url,
            custom_id,
            json,
        } => {
            commands::create_link::run(
                web_url,
                ios_deep_link,
                android_deep_link,
                ios_store_url,
                android_store_url,
                custom_id,
                json,
            )
            .await?
        }
        Command::TestLink { target, json } => commands::test_link::run(target, json).await?,
        Command::Doctor { json } => commands::doctor::run(json).await?,
        Command::Setup { command } => match command {
            SetupCommand::App {
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
                commands::setup_app::run(
                    platform,
                    bundle_id,
                    team_id,
                    package_name,
                    sha256_fingerprints,
                    app_name,
                    icon_url,
                    theme_color,
                    json,
                )
                .await?
            }
            SetupCommand::Domain {
                domain,
                provider,
                json,
            } => commands::setup_domain::run(domain, provider, json).await?,
        },
    }
    Ok(())
}
