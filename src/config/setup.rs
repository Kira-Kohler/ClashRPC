use inquire::{validator::Validation, Confirm, Password, Text};
use reqwest::blocking::Client as HttpClient;

use super::{save_config, AppConfig};
use crate::core::constants::VALID_ENVS;
use crate::core::embedded_env;
use crate::core::log::{log_info, log_ok, log_warn};
use crate::core::util::{die, is_release_mode, normalize_clan_tag, normalize_tag};
use crate::cr::api::fetch_player;
use crate::cr::models::Player;
use crate::ui::spinner;

pub fn verify_config(discord_client_id: &str, api_key: &str) {
    let rust_env = std::env::var("rust_env").unwrap_or_else(|_| {
        embedded_env::get("rust_env")
            .unwrap_or("production")
            .to_string()
    });
    let rust_env_lc = rust_env.to_lowercase();

    if !VALID_ENVS.contains(&rust_env_lc.as_str()) {
        let valids = VALID_ENVS
            .iter()
            .map(|e| format!("'{e}'"))
            .collect::<Vec<_>>()
            .join(", ");

        die(format!(
            "El entorno no es válido. Valores válidos: {}.",
            valids
        ));
    }

    if !discord_client_id.chars().all(|c| c.is_ascii_digit()) {
        die("El 'DISCORD_CLIENT_ID' no es válido.");
    }

    if api_key.trim().is_empty() {
        die("Falta la API key de Clash Royale.");
    }
}

pub fn get_api_key(cfg: &mut AppConfig) -> String {
    if is_release_mode() {
        if let Some(v) = cfg.clash_royale_api_key.as_deref() {
            let t = v.trim().to_string();
            if !t.is_empty() {
                return t;
            }
        }

        log_info("🔐 Necesito tu API Key de Clash Royale (https://developer.clashroyale.com/)");
        log_info("⚠️ Asegúrate de autorizar tu IP.\n");

        loop {
            let input_key = match Password::new("Pega tu API Key de Clash Royale:")
                .with_help_message(
                    "Se almacenará tu API Key localmente para facilitar su uso en el futuro.",
                )
                .without_confirmation()
                .prompt()
            {
                Ok(v) => v.trim().to_string(),
                Err(_) => die("Cancelado, no puedo seguir sin la API key."),
            };

            if input_key.is_empty() {
                log_warn("La API key no puede estar vacía.\n");
                continue;
            }

            cfg.clash_royale_api_key = Some(input_key.clone());
            save_config(cfg);
            log_ok("✅ API key guardada en config.json.\n");
            return input_key;
        }
    }

    if let Ok(v) = std::env::var("CLASH_ROYALE_API_KEY") {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return t;
        }
    }

    if let Some(v) = cfg.clash_royale_api_key.as_deref() {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return t;
        }
    }

    die("Falta la API Key de Clash Royale.");
}

pub fn prompt_player_tag(http: &HttpClient, cfg: &mut AppConfig) -> String {
    if let Some(saved) = cfg.player_tag.as_deref() {
        let clean = normalize_tag(saved);

        if !clean.is_empty() {
            let pb = spinner("Validando el TAG guardado en config.json...");
            match fetch_player(http, &clean) {
                Ok(p) => {
                    pb.finish_and_clear();
                    log_ok(format!("Usando el TAG guardado: {} ({})", p.name, p.tag));
                    println!();
                    return clean;
                }
                Err(e) => {
                    pb.finish_and_clear();
                    let status = e.status().map(|s| s.as_u16());
                    log_warn(format!(
                        "El TAG guardado no se pudo validar ({:?}), prueba de nuevo",
                        status
                    ));
                    println!();
                }
            }
        }
    }

    let env_tag = std::env::var("CLASH_ROYALE_PLAYER_TAG").unwrap_or_default();
    let default_clean = normalize_tag(&env_tag);
    let default_display = if default_clean.is_empty() {
        String::new()
    } else {
        format!("#{default_clean}")
    };

    log_info("📌 Necesito tu tag de Clash Royale para obtener tus stats.");
    log_info("   En el juego: Perfil → TAG debajo del nombre (tipo #ABC123).\n");

    loop {
        let mut prompt =
            Text::new("Pega tu TAG").with_help_message("Puedes ponerlo con o sin #. Ej: #ABC123");

        if !default_display.is_empty() {
            prompt = prompt.with_default(&default_display);
        }

        let raw = match prompt.prompt() {
            Ok(v) => v,
            Err(_) => die("Cancelado."),
        };

        let clean = normalize_tag(&raw);
        if clean.is_empty() {
            log_warn("No introduciste un TAG válido.\n");
            continue;
        }

        let pb = spinner("Chequeando TAG en la API...");
        let player = match fetch_player(http, &clean) {
            Ok(p) => {
                pb.finish_and_clear();
                p
            }
            Err(e) => {
                pb.finish_and_clear();
                let status = e.status().map(|s| s.as_u16());
                match status {
                    Some(404) => log_warn("Ese TAG no existe (404).".to_string()),
                    Some(401) | Some(403) => {
                        die("La API key no tiene permisos (401/403).");
                    }
                    _ => log_warn(format!("No pude validar el TAG ({:?}).", status)),
                }
                println!();
                continue;
            }
        };

        println!("\n✅ Jugador encontrado:");
        println!("  Nombre: {}", player.name);
        println!("  TAG: {}", player.tag);
        println!("  Copas: {}", player.trophies);
        println!();

        let ok = Confirm::new("¿Eres tú?")
            .with_default(true)
            .prompt()
            .unwrap_or(false);

        if ok {
            cfg.player_tag = Some(clean.clone());
            save_config(cfg);
            log_ok("TAG guardado en config.json.\n");
            return clean;
        }

        log_info("Vale, introduce un tag diferente:\n");
    }
}

pub fn prompt_clan_invite_link_if_needed(cfg: &mut AppConfig, player: &Player) {
    let Some(clan) = player.clan.as_ref() else {
        log_info("ℹ️ No estás en ningún clan ahora mismo.\n");
        cfg.clan_tag = None;
        cfg.clan_name = None;
        cfg.clan_invite_link = None;
        save_config(cfg);
        return;
    };

    let current_tag = normalize_clan_tag(&clan.tag);
    let saved_tag = cfg.clan_tag.as_deref().map(normalize_clan_tag);

    let changed = saved_tag.as_deref() != Some(current_tag.as_str());
    cfg.clan_tag = Some(current_tag.clone());
    cfg.clan_name = Some(clan.name.clone());

    log_info(format!(
        "🏰 Clan detectado: {} ({})",
        clan.name, current_tag
    ));

    if changed {
        cfg.clan_invite_link = None;
        save_config(cfg);
        log_warn("🔁 Detecté un cambio de clan o es el primer uso del botón.\n");
    } else {
        let has_valid_link = cfg
            .clan_invite_link
            .as_deref()
            .map(|s| s.trim().starts_with("https://"))
            .unwrap_or(false);

        if has_valid_link {
            log_ok("🔗 El enlace del clan ya está guardado.\n");
            save_config(cfg);
            return;
        }

        save_config(cfg);
    }

    let want = Confirm::new("¿Quieres un botón de “Unirse al clan” en Discord?")
        .with_default(false)
        .prompt()
        .unwrap_or(false);

    if !want {
        cfg.clan_invite_link = None;
        save_config(cfg);
        log_info("Vale, no pondré ningún botón de clan.\n");
        return;
    }

    let link = Text::new("Pega el enlace de invitación del clan")
        .with_help_message("En el juego: Clan → Invitar/Compartir → Copiar enlace")
        .with_validator(|input: &str| {
            let t = input.trim();
            if t.is_empty() {
                return Ok(Validation::Invalid("No puede estar vacío".into()));
            }
            if !t.starts_with("https://") {
                return Ok(Validation::Invalid("Tiene que empezar por https://".into()));
            }
            Ok(Validation::Valid)
        })
        .prompt();

    let link = match link {
        Ok(v) => v.trim().to_string(),
        Err(_) => {
            log_warn("Cancelado, no se guardó ningún enlace.\n");
            return;
        }
    };

    cfg.clan_invite_link = Some(link);
    save_config(cfg);
    log_ok("✅ Enlace guardado en config.json.\n");
}
