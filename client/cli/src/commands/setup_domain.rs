use console::style;
use dialoguer::{Input, Select};
use reqwest::StatusCode;
use tokio::time::{sleep, Duration};

use crate::config::StoredConfig;
use crate::ui;
use rift_client_core::apps::ListAppsResponse;
use rift_client_core::domains::{CreateDomainResponse, ListDomainsResponse};
use rift_client_core::error::RiftClientError;

const CLOUDFLARE_DASHBOARD_URL: &str = "https://dash.cloudflare.com/";
const WORKER_DOCS_URL: &str = "https://riftl.ink/docs/domains#cloudflare-worker";
const ROUTE_DOCS_URL: &str = "https://riftl.ink/docs/domains#worker-route";
const TEST_DOCS_URL: &str = "https://riftl.ink/docs/domains#test-custom-domain";
const WORKER_SOURCE_PATH: &str = "/Users/andreiterentiev/Developer/Relay/worker/src/index.js";

pub async fn run(
    domain: Option<String>,
    provider: Option<String>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = StoredConfig::load()?;
    let client =
        rift_client_core::RiftClient::with_secret_key(config.secret_key, Some(config.base_url));

    let provider = choose_provider(provider)?;
    if provider != "cloudflare" {
        return Err(
            "Only Cloudflare is supported right now. Use the manual custom domain docs for anything else."
                .into(),
        );
    }

    let setup_mode = choose_setup_mode()?;
    let root_domain = ask_root_domain(domain)?;
    let domains = client.list_domains().await?;

    if setup_mode == SetupMode::Alternate && !has_verified_primary(&domains) {
        ui::warning("Alternate domains require a verified primary domain first.");
        ui::note("Set up your main branded domain first, then come back to add the Open in App trampoline domain.");
        ui::code_line("rift setup domain");
        return Err("Alternate domains are blocked until a primary domain is verified.".into());
    }

    let direct_result = run_domain_flow(
        &client,
        &root_domain,
        setup_mode,
        &domains,
        json,
        FlowEntry::Direct,
    )
    .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&direct_result.json_payload)?
        );
        return Ok(());
    }

    if setup_mode == SetupMode::Primary {
        maybe_continue_to_alternate(&client, &root_domain).await?;
    } else if let Some(alternate) = direct_result.domain {
        ui::section("Domain Coverage");
        ui::success("Your alternate Open in App domain is ready.");
        ui::kv("Alternate", &alternate);
        ui::note("Your primary landing page can now use this domain for the cross-domain Open in App handoff.");
    }

    Ok(())
}

async fn maybe_continue_to_alternate(
    client: &rift_client_core::RiftClient,
    root_domain: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let domains = client.list_domains().await?;
    let primary = domains
        .domains
        .iter()
        .find(|domain| domain.role == "primary" && domain.verified)
        .map(|domain| domain.domain.clone());
    let alternate = domains
        .domains
        .iter()
        .find(|domain| domain.role == "alternate")
        .map(|domain| ExistingAlternate {
            domain: domain.domain.clone(),
            verified: domain.verified,
        });

    if let Some(alternate) = alternate.as_ref().filter(|domain| domain.verified) {
        ui::section("Domain Coverage");
        ui::success("Your primary and alternate domains are both set up.");
        if let Some(primary) = primary {
            ui::kv("Primary", primary);
        }
        ui::kv("Alternate", &alternate.domain);
        return Ok(());
    }

    let apps = client
        .list_apps()
        .await
        .unwrap_or(ListAppsResponse { apps: Vec::new() });

    ui::section("Recommended Next Step");
    ui::note("An alternate domain improves iOS Open in App reliability by acting as the cross-domain trampoline.");
    if apps.apps.iter().any(|app| app.platform == "ios") {
        ui::note("You already have an iOS app configured, so this is worth finishing now.");
    } else {
        ui::note("Even if iOS app setup comes later, getting the alternate domain done now saves a round-trip.");
    }

    if !ui::choose(
        "Set up the recommended alternate domain now?",
        "Yes, continue with alternate setup",
        "No, I'll do this later",
        true,
    )? {
        return Ok(());
    }

    let _ = run_domain_flow(
        client,
        root_domain,
        SetupMode::Alternate,
        &domains,
        false,
        FlowEntry::FollowThrough,
    )
    .await?;

    let final_domains = client.list_domains().await?;
    let primary = final_domains
        .domains
        .iter()
        .find(|domain| domain.role == "primary" && domain.verified)
        .map(|domain| domain.domain.clone());
    let alternate = final_domains
        .domains
        .iter()
        .find(|domain| domain.role == "alternate" && domain.verified)
        .map(|domain| domain.domain.clone());

    ui::section("Domain Coverage");
    ui::success("Your primary and alternate domains are both ready.");
    if let Some(primary) = primary {
        ui::kv("Primary", primary);
    }
    if let Some(alternate) = alternate {
        ui::kv("Alternate", alternate);
    }

    Ok(())
}

async fn run_domain_flow(
    client: &rift_client_core::RiftClient,
    root_domain: &str,
    mode: SetupMode,
    existing_domains: &ListDomainsResponse,
    json: bool,
    entry: FlowEntry,
) -> Result<DomainFlowResult, Box<dyn std::error::Error>> {
    let existing_alternate = existing_domains
        .domains
        .iter()
        .find(|domain| domain.role == "alternate")
        .map(|domain| ExistingAlternate {
            domain: domain.domain.clone(),
            verified: domain.verified,
        });

    let target_domain = match mode {
        SetupMode::Primary => choose_target_domain(root_domain, mode, None)?,
        SetupMode::Alternate => {
            if let Some(existing_alternate) = existing_alternate.as_ref() {
                if json {
                    existing_alternate.domain.clone()
                } else {
                    ui::section("Alternate Domain");
                    ui::note(&format!(
                        "Rift already has an alternate domain record for `{}`. Continuing with that hostname.",
                        existing_alternate.domain
                    ));
                    existing_alternate.domain.clone()
                }
            } else {
                choose_target_domain(root_domain, mode, None)?
            }
        }
    };

    let created = match client
        .create_domain(&rift_client_core::domains::CreateDomainRequest {
            domain: target_domain.clone(),
            role: Some(mode.api_role().to_string()),
        })
        .await
    {
        Ok(created) => DomainSetupState::Created(created),
        Err(RiftClientError::Api { status, message })
            if status == 409 && message.contains("Domain already registered") =>
        {
            DomainSetupState::Existing {
                domain: target_domain.clone(),
            }
        }
        Err(RiftClientError::Api { status, message })
            if status == 409
                && mode == SetupMode::Alternate
                && message.contains("Only one alternate domain allowed") =>
        {
            let existing = existing_alternate
                .as_ref()
                .map(|domain| domain.domain.as_str())
                .unwrap_or("another alternate domain");
            return Err(format!(
                "Only one alternate domain is allowed per team. Rift already has `{existing}` registered."
            )
            .into());
        }
        Err(error) => return Err(error.into()),
    };

    let result = DomainFlowResult {
        domain: Some(created.domain().to_string()),
        json_payload: json_payload(&created, root_domain, mode),
    };

    if json {
        return Ok(result);
    }

    print_dns_step(&created, root_domain, mode, entry)?;

    let created_domain = created.domain().to_string();
    verify_domain_loop(
        client,
        &created_domain,
        matches!(created, DomainSetupState::Existing { .. }),
        mode,
    )
    .await?;

    manual_worker_setup_flow(&created_domain, mode).await?;

    Ok(result)
}

fn choose_provider(provider: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
    Ok(match provider {
        Some(provider) => provider,
        None => {
            let idx = Select::with_theme(&ui::theme())
                .with_prompt("Choose your DNS provider")
                .items(&["Cloudflare", "Other (coming soon)"])
                .default(0)
                .interact()?;
            ["cloudflare", "other"][idx].to_string()
        }
    })
}

fn choose_setup_mode() -> Result<SetupMode, Box<dyn std::error::Error>> {
    let options = [
        "Primary branded domain — your main public link domain",
        "Alternate domain — more reliable Open in App behavior",
    ];
    let idx = Select::with_theme(&ui::theme())
        .with_prompt("What do you want to set up?")
        .items(&options)
        .default(0)
        .interact()?;
    Ok(match idx {
        0 => SetupMode::Primary,
        _ => SetupMode::Alternate,
    })
}

fn ask_root_domain(domain: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
    let main_site_input: String = Input::with_theme(&ui::theme())
        .with_prompt("What is your main website?")
        .default(domain.unwrap_or_default())
        .validate_with(|input: &String| -> Result<(), &str> {
            (!input.trim().is_empty())
                .then_some(())
                .ok_or("Enter a domain like bitcoin.com")
        })
        .interact_text()?;
    Ok(normalize_host(&main_site_input))
}

fn choose_target_domain(
    root_domain: &str,
    mode: SetupMode,
    current_domain: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let recommended = current_domain
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{}.{root_domain}", mode.default_subdomain()));

    let use_recommended = ui::choose(
        &format!(
            "Use the recommended {} domain `{recommended}`?",
            mode.label().to_lowercase()
        ),
        "Yes, use the recommendation",
        "No, choose a different domain",
        true,
    )?;

    Ok(if use_recommended {
        recommended
    } else {
        let custom: String = Input::with_theme(&ui::theme())
            .with_prompt(format!(
                "Which {} domain should Rift use?",
                mode.label().to_lowercase()
            ))
            .validate_with(|input: &String| -> Result<(), &str> {
                (!input.trim().is_empty())
                    .then_some(())
                    .ok_or("Enter a domain like links.bitcoin.com")
            })
            .interact_text()?;
        normalize_host(&custom)
    })
}

fn print_dns_step(
    created: &DomainSetupState,
    root_domain: &str,
    mode: SetupMode,
    entry: FlowEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    match created {
        DomainSetupState::Created(created) => {
            println!();
            println!(
                "{}",
                style(format!(
                    "Step 1: add the {} TXT record in Cloudflare DNS",
                    mode.label().to_lowercase()
                ))
                .bold()
                .cyan()
            );
            println!();
            ui::kv("Type", "TXT");
            ui::kv(
                "Name",
                display_record_name(&created.txt_record, root_domain),
            );
            ui::kv("Value", &created.verification_token);
            ui::spacer();
            ui::note(mode.dns_note());

            if ui::choose(
                "Open Cloudflare DNS in your browser now?",
                "Yes, open Cloudflare",
                "No, I'll do it myself",
                true,
            )? {
                let _ = webbrowser::open(CLOUDFLARE_DASHBOARD_URL);
            }
        }
        DomainSetupState::Existing { domain } => {
            ui::spacer();
            ui::warning(&format!("Rift already knows about `{domain}`."));
            match (mode, entry) {
                (SetupMode::Alternate, FlowEntry::FollowThrough) => {
                    ui::note("Continuing the existing alternate-domain setup so you can finish verification and testing.");
                }
                _ => ui::note("Continuing from the verification step so you can finish setup."),
            }
        }
    }

    Ok(())
}

async fn verify_domain_loop(
    client: &rift_client_core::RiftClient,
    domain: &str,
    is_resume: bool,
    mode: SetupMode,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if is_resume {
            println!();
            println!(
                "{}",
                style(format!(
                    "Step 1: verify the {} TXT record with Rift",
                    mode.label().to_lowercase()
                ))
                .bold()
                .cyan()
            );
            ui::note("If you already added the Cloudflare TXT record earlier, Rift can pick up from there.");
        } else if !ui::choose(
            "Have you added the TXT record in Cloudflare?",
            "Yes, verify it now",
            "No, I'll come back later",
            true,
        )? {
            return Err(
                "Add the TXT record first, then rerun `rift setup domain` when you're ready."
                    .into(),
            );
        }

        match client.verify_domain(domain).await {
            Ok(response) if response.verified => {
                ui::spacer();
                ui::success("Domain verified with Rift.");
                return Ok(());
            }
            _ => {
                ui::spacer();
                ui::warning("Rift could not verify the TXT record yet.");
                ui::note("Cloudflare DNS may still be propagating.");
                if !ui::choose(
                    "Try verification again?",
                    "Yes, try again",
                    "No, stop here",
                    true,
                )? {
                    if is_resume {
                        return Err(
                            "Verification is still pending. The domain already exists in Rift, so rerun `rift setup domain` after the TXT record is live."
                                .into(),
                        );
                    }
                    return Err(
                        "Verification is still pending. Rerun `rift setup domain` in a minute."
                            .into(),
                    );
                }
            }
        }
    }
}

async fn manual_worker_setup_flow(
    domain: &str,
    mode: SetupMode,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!(
        "{}",
        style(format!(
            "Step 2: finish the {} Cloudflare Worker setup manually",
            mode.label().to_lowercase()
        ))
        .bold()
        .cyan()
    );
    println!();
    println!(
        "{} {}",
        style("Open this docs section:").bold(),
        style(WORKER_DOCS_URL).underlined().cyan()
    );
    println!();
    println!("{}", style("Cloudflare checklist").bold());
    println!(
        "  1. Create or reuse a Worker like {}",
        style("rift-proxy").cyan()
    );
    println!(
        "  2. Paste the proxy code from {}",
        style(WORKER_SOURCE_PATH).dim()
    );
    println!("  3. Add {} as a Custom Domain", style(domain).cyan());
    println!(
        "  4. Add {} as a Route",
        style(format!("{domain}/*")).cyan()
    );
    println!();
    ui::note(mode.worker_note());
    println!();
    println!("{}", style("Quick check").bold());
    println!(
        "  {}",
        style(format!("curl https://{domain}/llms.txt")).green()
    );
    println!("  This should return Relay's llms.txt through your custom domain.");
    println!();
    println!(
        "{} {}",
        style("More docs:").bold(),
        style(ROUTE_DOCS_URL).dim()
    );
    println!(
        "{} {}",
        style("Test commands:").bold(),
        style(TEST_DOCS_URL).dim()
    );

    if ui::choose(
        "Open the Worker setup docs now?",
        "Yes, open the docs",
        "No, keep this in the terminal",
        true,
    )? {
        let _ = webbrowser::open(WORKER_DOCS_URL);
    }

    ui::section("Finish Setup");
    ui::note(
        "Come back here after you have attached the Worker, Custom Domain, and wildcard Route.",
    );

    if !ui::choose(
        "Finished setting up the Worker in Cloudflare?",
        "Yes, run the diagnostic",
        "No, I'll come back later",
        true,
    )? {
        return Err("Finish the Worker setup in Cloudflare, then rerun `rift setup domain` when you're ready to test it.".into());
    }

    run_worker_diagnostic(domain, mode).await?;

    Ok(())
}

async fn run_worker_diagnostic(
    domain: &str,
    mode: SetupMode,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        ui::section("Worker Diagnostic");
        ui::note("Rift is checking whether your custom domain is proxying traffic correctly.");
        ui::code_line(format!("curl https://{domain}/llms.txt"));

        match wait_for_worker_probe(domain).await {
            Ok(ProbeResult::Success) => {
                ui::success("Your Worker is responding through the custom domain.");
                if mode == SetupMode::Alternate {
                    ui::note("Your primary landing page can now use this alternate domain for the Open in App handoff.");
                }
                return Ok(());
            }
            Ok(ProbeResult::BadStatus(status)) => {
                ui::warning(&format!(
                    "The custom domain responded, but with HTTP {} instead of a clean Rift response.",
                    status.as_u16()
                ));
                ui::note("This usually means the Worker, Custom Domain, or Route is only partially configured.");
            }
            Ok(ProbeResult::UnexpectedBody) => {
                ui::warning("The custom domain responded, but it does not look like the Rift Worker is proxying Relay yet.");
                ui::note("Double-check the Worker code and the wildcard Route.");
            }
            Err(error) => {
                ui::warning(&format!(
                    "Rift could not reach https://{domain}/llms.txt yet."
                ));
                ui::note(&error);
            }
        }

        if !ui::choose(
            "Try the diagnostic again?",
            "Yes, try again",
            "No, stop here",
            true,
        )? {
            return Err(
                "The Worker diagnostic is still failing. Finish the Cloudflare setup and rerun `rift setup domain` to test again."
                    .into(),
            );
        }
    }
}

async fn wait_for_worker_probe(domain: &str) -> Result<ProbeResult, String> {
    let attempts = 4;
    for attempt in 1..=attempts {
        match fetch_worker_probe(domain).await {
            Ok(ProbeResult::Success) => return Ok(ProbeResult::Success),
            Ok(other) if attempt == attempts => return Ok(other),
            Err(error) if attempt == attempts => return Err(error),
            Ok(_) | Err(_) => {
                ui::note(&format!(
                    "Cloudflare may still be propagating. Checking again in 30 seconds ({attempt}/{attempts})..."
                ));
                sleep(Duration::from_secs(30)).await;
            }
        }
    }

    Err("Worker probe did not complete.".to_string())
}

async fn fetch_worker_probe(domain: &str) -> Result<ProbeResult, String> {
    let url = format!("https://{domain}/llms.txt");
    let response = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if response.status() != StatusCode::OK {
        return Ok(ProbeResult::BadStatus(response.status()));
    }

    let body = response.text().await.map_err(|error| error.to_string())?;
    if body.contains("Deep links for humans and agents") || body.contains("Rift routes users") {
        Ok(ProbeResult::Success)
    } else {
        Ok(ProbeResult::UnexpectedBody)
    }
}

fn has_verified_primary(domains: &ListDomainsResponse) -> bool {
    domains
        .domains
        .iter()
        .any(|domain| domain.role == "primary" && domain.verified)
}

fn json_payload(
    created: &DomainSetupState,
    root_domain: &str,
    mode: SetupMode,
) -> serde_json::Value {
    match created {
        DomainSetupState::Created(created) => serde_json::json!({
            "domain": created.domain,
            "provider": "cloudflare",
            "role": mode.api_role(),
            "txt_record": display_record_name(&created.txt_record, root_domain),
            "verification_token": created.verification_token,
            "resume": false,
            "manual_worker_setup": true,
        }),
        DomainSetupState::Existing { domain } => serde_json::json!({
            "domain": domain,
            "provider": "cloudflare",
            "role": mode.api_role(),
            "resume": true,
            "manual_worker_setup": true,
        }),
    }
}

fn display_record_name(full_name: &str, root_domain: &str) -> String {
    let suffix = format!(".{root_domain}");
    full_name
        .strip_suffix(&suffix)
        .unwrap_or(full_name)
        .to_string()
}

fn normalize_host(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_matches('/')
        .to_string()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SetupMode {
    Primary,
    Alternate,
}

impl SetupMode {
    fn label(self) -> &'static str {
        match self {
            Self::Primary => "Primary branded domain",
            Self::Alternate => "Alternate Open in App domain",
        }
    }

    fn api_role(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Alternate => "alternate",
        }
    }

    fn default_subdomain(self) -> &'static str {
        match self {
            Self::Primary => "go",
            Self::Alternate => "open",
        }
    }

    fn dns_note(self) -> &'static str {
        match self {
            Self::Primary => {
                "This is the only DNS record Rift needs for primary domain verification."
            }
            Self::Alternate => {
                "This alternate domain uses the same verification flow as your primary domain."
            }
        }
    }

    fn worker_note(self) -> &'static str {
        match self {
            Self::Primary => "This is your main branded landing-page domain.",
            Self::Alternate => {
                "This domain is only for the cross-domain Open in App handoff. It is not your main branded landing-page domain."
            }
        }
    }
}

#[derive(Clone, Copy)]
enum FlowEntry {
    Direct,
    FollowThrough,
}

enum DomainSetupState {
    Created(CreateDomainResponse),
    Existing { domain: String },
}

impl DomainSetupState {
    fn domain(&self) -> &str {
        match self {
            Self::Created(created) => &created.domain,
            Self::Existing { domain } => domain,
        }
    }
}

enum ProbeResult {
    Success,
    BadStatus(StatusCode),
    UnexpectedBody,
}

struct DomainFlowResult {
    domain: Option<String>,
    json_payload: serde_json::Value,
}

struct ExistingAlternate {
    domain: String,
    verified: bool,
}
