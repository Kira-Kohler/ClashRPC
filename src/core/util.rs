use std::{env, process};

use super::embedded_env;
use super::log::log_err;

pub fn die(msg: impl AsRef<str>) -> ! {
    log_err(msg.as_ref());
    process::exit(1);
}

pub fn truthy_env(v: &str) -> bool {
    let t = v.trim().to_lowercase();
    t == "1" || t == "true" || t == "yes" || t == "y"
}

pub fn is_release_mode() -> bool {
    if let Ok(v) = env::var("rust_env") {
        return v.trim().eq_ignore_ascii_case("release");
    }

    if let Some(v) = embedded_env::get("rust_env") {
        return v.trim().eq_ignore_ascii_case("release");
    }

    !cfg!(debug_assertions)
}

pub fn get_required_env(key: &str) -> String {
    if let Ok(v) = env::var(key) {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return t;
        }
    }

    if let Some(v) = embedded_env::get(key) {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return t;
        }
    }

    die(format!(
        "Falta '{key}', no está en el entorno ni embebido en el exe."
    ))
}

pub fn get_optional_env(key: &str) -> Option<String> {
    if let Ok(v) = env::var(key) {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }

    if let Some(v) = embedded_env::get(key) {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }

    None
}

pub fn normalize_tag(tag: &str) -> String {
    tag.trim()
        .trim_start_matches('#')
        .to_uppercase()
        .split_whitespace()
        .collect::<String>()
}

pub fn normalize_clan_tag(tag: &str) -> String {
    let t = tag.trim().trim_start_matches('#').to_uppercase();
    format!("#{t}")
}

pub fn parse_u64_env(key: &str, default: u64) -> u64 {
    match env::var(key)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
    {
        Some(v) if v > 0 => v,
        _ => default,
    }
}
