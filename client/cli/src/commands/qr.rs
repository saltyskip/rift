use std::fs;

use crate::config::StoredConfig;
use crate::error::CliError;
use crate::ui;

pub struct Args {
    pub target: String,
    pub format: String,
    pub output: String,
    pub logo: Option<String>,
    pub size: Option<u32>,
    pub level: Option<String>,
    pub fg_color: Option<String>,
    pub bg_color: Option<String>,
    pub hide_logo: bool,
    pub margin: Option<u32>,
    pub json: bool,
}

pub async fn run(args: Args) -> Result<(), CliError> {
    let link_id = args
        .target
        .rsplit('/')
        .next()
        .unwrap_or(&args.target)
        .to_string();
    let config = StoredConfig::load().map_err(|_| CliError::AuthFailed)?;
    let client =
        rift_client_core::RiftClient::with_secret_key(config.secret_key, Some(config.base_url));
    let options = rift_client_core::links::QrCodeOptions {
        logo: args.logo,
        size: args.size,
        level: args.level,
        fg_color: args.fg_color,
        bg_color: args.bg_color,
        hide_logo: args.hide_logo,
        margin: args.margin,
    };

    let bytes = match args.format.as_str() {
        "png" => client.get_link_qr_png(&link_id, &options).await?,
        "svg" => client.get_link_qr_svg(&link_id, &options).await?,
        _ => {
            return Err(CliError::General(
                "format must be either 'png' or 'svg'".to_string(),
            ));
        }
    };
    fs::write(&args.output, bytes)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "link_id": link_id,
                "format": args.format,
                "output": args.output,
            }))?
        );
    } else {
        ui::heading("QR Code Saved", "Your styled QR code is ready.");
        ui::kv("Link ID", &link_id);
        ui::kv("Format", &args.format);
        ui::kv("Output", &args.output);
    }

    Ok(())
}
