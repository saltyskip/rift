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
    /// Manage team members on your tenant
    #[command(subcommand)]
    Team(TeamCommand),
    /// Start or upgrade a paid subscription (opens Stripe in browser)
    Subscribe {
        /// One of: pro, business, scale
        tier: String,
        #[arg(long)]
        json: bool,
    },
    /// Cancel your subscription at current_period_end
    Cancel {
        #[arg(long)]
        json: bool,
    },
    /// Show plan tier, limits, and renewal date
    Billing {
        #[arg(long)]
        json: bool,
    },
    /// Print the CLI version and build target
    Version {
        #[arg(long)]
        json: bool,
    },
    /// Download and install the latest CLI release
    Update {
        /// Show whether a newer version exists without installing
        #[arg(long)]
        check: bool,
        /// Install a specific version (e.g. 0.1.3) instead of the latest
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        json: bool,
    },
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
        preview_title: Option<String>,
        #[arg(long)]
        preview_description: Option<String>,
        #[arg(long)]
        preview_image_url: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Download a styled QR code for a deep link
    Qr {
        target: String,
        #[arg(long, default_value = "png")]
        format: String,
        #[arg(long)]
        output: String,
        #[arg(long)]
        logo: Option<String>,
        #[arg(long)]
        size: Option<u32>,
        #[arg(long)]
        level: Option<String>,
        #[arg(long)]
        fg_color: Option<String>,
        #[arg(long)]
        bg_color: Option<String>,
        #[arg(long)]
        hide_logo: bool,
        #[arg(long)]
        margin: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Preview how a link resolves across platforms
    Test {
        target: String,
        #[arg(long)]
        json: bool,
    },
    /// Funnel stats for one or more links (clicks → installs → identifies → conversions)
    Stats {
        /// Link IDs, comma-separated for multiple (e.g. ABC123,DEF456)
        link_ids: String,
        /// Start of date range, RFC 3339 (default: 30 days ago)
        #[arg(long)]
        from: Option<String>,
        /// End of date range, RFC 3339 (default: now)
        #[arg(long)]
        to: Option<String>,
        /// Attribution model: last_touch (default), first_touch, or touched
        #[arg(long)]
        credit: Option<String>,
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
    /// Add and verify a custom domain with guided DNS setup
    Setup {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum TeamCommand {
    /// Invite a teammate by email
    Invite {
        /// Email address of the person to invite
        email: String,
        #[arg(long)]
        json: bool,
    },
    /// List team members on this tenant
    List {
        #[arg(long)]
        json: bool,
    },
    /// Remove a team member (by email or 24-char user id)
    Remove {
        /// Email address or user id to remove
        who: String,
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
                preview_title,
                preview_description,
                preview_image_url,
                json,
            } => {
                commands::create_link::run(commands::create_link::Args {
                    web_url,
                    ios_deep_link,
                    android_deep_link,
                    ios_store_url,
                    android_store_url,
                    custom_id,
                    preview_title,
                    preview_description,
                    preview_image_url,
                    json,
                })
                .await
            }
            LinksCommand::Qr {
                target,
                format,
                output,
                logo,
                size,
                level,
                fg_color,
                bg_color,
                hide_logo,
                margin,
                json,
            } => {
                commands::qr::run(commands::qr::Args {
                    target,
                    format,
                    output,
                    logo,
                    size,
                    level,
                    fg_color,
                    bg_color,
                    hide_logo,
                    margin,
                    json,
                })
                .await
            }
            LinksCommand::Test { target, json } => commands::test_link::run(target, json).await,
            LinksCommand::Stats {
                link_ids,
                from,
                to,
                credit,
                json,
            } => {
                commands::stats::run(commands::stats::Args {
                    link_ids,
                    from,
                    to,
                    credit,
                    json,
                })
                .await
            }
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
            DomainsCommand::Setup { domain, json } => {
                commands::setup_domain::run(domain, json).await
            }
        },
        Command::Team(cmd) => match cmd {
            TeamCommand::Invite { email, json } => commands::team_invite::run(email, json).await,
            TeamCommand::List { json } => commands::team_list::run(json).await,
            TeamCommand::Remove { who, json } => commands::team_remove::run(who, json).await,
        },
        Command::Subscribe { tier, json } => commands::subscribe::run(tier, json).await,
        Command::Cancel { json } => commands::cancel::run(json).await,
        Command::Billing { json } => commands::billing::run(json).await,
        Command::Version { json } => commands::version::run(json),
        Command::Update {
            check,
            version,
            json,
        } => {
            commands::update::run(commands::update::Args {
                check,
                version,
                json,
            })
            .await
        }
        Command::Completions { shell } => commands::completions::run(shell),
    }
}
