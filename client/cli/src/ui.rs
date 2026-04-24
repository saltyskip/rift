use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

pub fn spacer() {
    println!();
}

pub fn heading(title: &str, subtitle: &str) {
    println!("{}", style(title).bold().cyan());
    if !subtitle.is_empty() {
        println!("{}", style(subtitle).dim());
    }
}

pub fn section(title: &str) {
    println!();
    println!("{}", style(title).bold().underlined());
}

pub fn success(message: &str) {
    println!("{}", style(message).green().bold());
}

pub fn warning(message: &str) {
    println!("{}", style(message).yellow().bold());
}

pub fn note(message: &str) {
    println!("{}", style(message).dim());
}

pub fn kv(label: &str, value: impl AsRef<str>) {
    println!(
        "{} {}",
        style(format!("{label}:")).bold(),
        style(value.as_ref()).cyan()
    );
}

pub fn bullet(message: impl AsRef<str>) {
    println!("  {} {}", style("•").cyan(), message.as_ref());
}

pub fn numbered(index: usize, message: impl AsRef<str>) {
    println!(
        "  {} {}",
        style(format!("{index}.")).cyan().bold(),
        message.as_ref()
    );
}

pub fn code_line(text: impl AsRef<str>) {
    println!("  {}", style(text.as_ref()).green());
}

pub fn badge(label: &str, title: &str, description: &str, done: bool) {
    let badge = if done {
        style(format!("[{label}]")).green().bold()
    } else {
        style(format!("[{label}]")).yellow().bold()
    };
    println!("  {} {}", badge, style(title).bold());
    println!("      {}", style(description).dim());
}

pub fn spinner(message: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(message.into());
    pb
}

pub fn choose(
    prompt: &str,
    yes_label: &str,
    no_label: &str,
    default_yes: bool,
) -> Result<bool, dialoguer::Error> {
    let options = [yes_label, no_label];
    let idx = Select::with_theme(&theme())
        .with_prompt(prompt)
        .items(&options)
        .default(if default_yes { 0 } else { 1 })
        .interact()?;
    Ok(idx == 0)
}
