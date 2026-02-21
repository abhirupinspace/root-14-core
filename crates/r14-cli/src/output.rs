use std::sync::atomic::{AtomicBool, Ordering};

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

static JSON_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_json_mode(enabled: bool) {
    JSON_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_json() -> bool {
    JSON_MODE.load(Ordering::Relaxed)
}

pub fn success(msg: &str) {
    if !is_json() {
        eprintln!("{}", msg.green());
    }
}

pub fn warn(msg: &str) {
    if !is_json() {
        eprintln!("{}", msg.yellow());
    }
}

pub fn error_msg(msg: &str) {
    if !is_json() {
        eprintln!("{}", msg.red());
    }
}

pub fn info(msg: &str) {
    if !is_json() {
        eprintln!("{}", msg);
    }
}

pub fn label(key: &str, val: &str) {
    if !is_json() {
        eprintln!("{} {}", format!("{}:", key).bold(), val);
    }
}

pub fn json_output(value: serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(&value).unwrap());
}

pub fn spinner(msg: &str) -> ProgressBar {
    if is_json() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

pub fn fail_with_hint(error: &str, hint: &str) -> anyhow::Error {
    anyhow::anyhow!("{}\n{} {}", error.red(), "hint:".bold(), hint)
}
