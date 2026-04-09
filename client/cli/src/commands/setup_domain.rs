use console::style;
use dialoguer::{Input, Select};
use tokio::time::{sleep, Duration};

use crate::context;
use crate::error::CliError;
use crate::ui;
use rift_client_core::apps::ListAppsResponse;
use rift_client_core::domains::{CreateDomainResponse, ListDomainsResponse};
use rift_client_core::error::RiftClientError;

const DOMAIN_DOCS_URL: &str = "https://riftl.ink/docs/domains";

pub async fn run(
    domain: Option<String>,
    provider: Option<String>,
    json: bool,
) -> Result<(), CliError> {
    let client = context::authenticated_client()?;
    let _ = provider; // Provider no longer gated — any DNS provider works.

    let setup_mode = choose_setup_mode()?;
    let root_domain = ask_root_domain(domain)?;
    let domains = client.list_domains().await?;

    if setup_mode == SetupMode::Alternate && !has_verified_primary(&domains) {
        ui::warning("Alternate domains require a verified primary domain first.");
        ui::note("Set up your main branded domain first, then come back to add the Open in App trampoline domain.");
        ui::code_line("rift domains setup");
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
) -> Result<(), CliError> {
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
) -> Result<DomainFlowResult, CliError> {
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

    cname_setup_flow(&created, &created_domain, mode).await?;

    Ok(result)
}

fn choose_setup_mode() -> Result<SetupMode, CliError> {
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

fn ask_root_domain(domain: Option<String>) -> Result<String, CliError> {
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
) -> Result<String, CliError> {
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
) -> Result<(), CliError> {
    match created {
        DomainSetupState::Created(created) => {
            println!();
            println!(
                "{}",
                style(format!(
                    "Step 1: add these DNS records for your {} domain",
                    mode.label().to_lowercase()
                ))
                .bold()
                .cyan()
            );
            println!();
            ui::kv(
                "CNAME",
                format!("{} → {}", &created.domain, &created.cname_target),
            );
            ui::kv(
                "TXT",
                format!(
                    "{} → {}",
                    display_record_name(&created.txt_record, root_domain),
                    &created.verification_token
                ),
            );
            ui::spacer();
            ui::note(mode.dns_note());
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
) -> Result<(), CliError> {
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
            ui::note("If you already added the TXT record, Rift can pick up from there.");
        } else if !ui::choose(
            "Have you added the DNS records?",
            "Yes, verify it now",
            "No, I'll come back later",
            true,
        )? {
            return Err(
                "Add the TXT record first, then rerun `rift domains setup` when you're ready."
                    .into(),
            );
        }

        match client.verify_domain(domain).await {
            Ok(response) if response.verified => {
                ui::spacer();
                ui::success("Domain verified with Rift.");

                // Wait for TLS certificate if Fly is configured.
                if !response.tls.is_empty() && response.tls != "none" {
                    wait_for_tls(client, domain).await;
                }

                return Ok(());
            }
            _ => {
                ui::spacer();
                ui::warning("Rift could not verify the TXT record yet.");
                ui::note("DNS may still be propagating.");
                if !ui::choose(
                    "Try verification again?",
                    "Yes, try again",
                    "No, stop here",
                    true,
                )? {
                    if is_resume {
                        return Err(
                            "Verification is still pending. The domain already exists in Rift, so rerun `rift domains setup` after the TXT record is live."
                                .into(),
                        );
                    }
                    return Err(
                        "Verification is still pending. Rerun `rift domains setup` in a minute."
                            .into(),
                    );
                }
            }
        }
    }
}

async fn wait_for_tls(client: &rift_client_core::RiftClient, domain: &str) {
    ui::note("Waiting for TLS certificate...");
    let max_attempts = 12; // 60 seconds total
    for _ in 0..max_attempts {
        sleep(Duration::from_secs(5)).await;
        match client.verify_domain(domain).await {
            Ok(resp) if resp.tls == "active" => {
                ui::success("TLS certificate is active.");
                return;
            }
            Ok(resp) if resp.tls == "failed" => {
                ui::warning("TLS certificate provisioning failed. Check that your CNAME is pointing correctly.");
                return;
            }
            _ => {}
        }
    }
    ui::warning("TLS certificate is still provisioning. It may take a few more minutes.");
    ui::note("You can check later with: rift domains setup");
}

async fn cname_setup_flow(
    created: &DomainSetupState,
    domain: &str,
    mode: SetupMode,
) -> Result<(), CliError> {
    // Show the CNAME target if we have it from the create response.
    if let DomainSetupState::Created(resp) = created {
        println!();
        println!(
            "{}",
            style("Step 2: verify your custom domain is reachable")
                .bold()
                .cyan()
        );
        println!();
        ui::note(&format!(
            "Make sure your CNAME record is pointing {} → {}",
            domain, resp.cname_target
        ));
    }

    if !ui::choose(
        "Have you added the CNAME record?",
        "Yes, test connectivity",
        "No, I'll come back later",
        true,
    )? {
        ui::note(&format!(
            "Add the CNAME, then rerun `rift domains setup`. Docs: {}",
            DOMAIN_DOCS_URL
        ));
        return Ok(());
    }

    run_connectivity_check(domain, mode).await
}

async fn run_connectivity_check(domain: &str, mode: SetupMode) -> Result<(), CliError> {
    loop {
        ui::note("Checking whether your custom domain is reachable...");

        match fetch_domain_probe(domain).await {
            Ok(true) => {
                ui::success("Your custom domain is responding correctly.");
                if mode == SetupMode::Alternate {
                    ui::note("Your primary landing page can now use this alternate domain for the Open in App handoff.");
                }
                return Ok(());
            }
            Ok(false) => {
                ui::warning(
                    "The domain responded, but doesn't appear to be serving Rift content yet.",
                );
                ui::note("DNS and TLS provisioning can take a few minutes.");
            }
            Err(error) => {
                ui::warning(&format!("Could not reach https://{domain}/llms.txt yet."));
                ui::note(&error);
            }
        }

        if !ui::choose("Try again?", "Yes, try again", "No, stop here", true)? {
            ui::note("You can test later with:");
            ui::code_line(format!("curl https://{domain}/llms.txt"));
            return Ok(());
        }

        sleep(Duration::from_secs(5)).await;
    }
}

async fn fetch_domain_probe(domain: &str) -> Result<bool, String> {
    let url = format!("https://{domain}/llms.txt");
    let response = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Ok(false);
    }

    let body = response.text().await.map_err(|error| error.to_string())?;
    Ok(body.contains("Deep links for humans and agents") || body.contains("Rift routes users"))
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
            "role": mode.api_role(),
            "cname_target": created.cname_target,
            "txt_record": display_record_name(&created.txt_record, root_domain),
            "verification_token": created.verification_token,
            "resume": false,
        }),
        DomainSetupState::Existing { domain } => serde_json::json!({
            "domain": domain,
            "role": mode.api_role(),
            "resume": true,
        }),
    }
}

pub fn display_record_name(full_name: &str, root_domain: &str) -> String {
    let suffix = format!(".{root_domain}");
    full_name
        .strip_suffix(&suffix)
        .unwrap_or(full_name)
        .to_string()
}

pub fn normalize_host(value: &str) -> String {
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

struct DomainFlowResult {
    domain: Option<String>,
    json_payload: serde_json::Value,
}

struct ExistingAlternate {
    domain: String,
    verified: bool,
}
