use std::time::Instant;
use std::{env, thread, time::Duration};

use reqwest::blocking::Client as HttpClient;

use crate::config::setup::{
    get_api_key, prompt_clan_invite_link_if_needed, prompt_player_tag, verify_config,
};
use crate::config::{config_path, load_config};
use crate::core::constants::APP_NAME;
use crate::core::embedded_env;
use crate::core::log::{log_info, log_warn};
use crate::core::util::{get_optional_env, get_required_env, parse_u64_env, truthy_env};
use crate::cr::api::{build_http_client, fetch_last_battle, fetch_player, probe_image_url};
use crate::cr::arena::{
    arena_base_url, arena_id_field_to_number, arena_id_to_number, arena_name_to_number,
    arena_raw_name_to_number, arena_url,
};
use crate::cr::models::{BattleLogEntry, Player};
use crate::discord::rpc::{build_activity, connect_rpc_with_retry};
use crate::ui::{banner, set_window_title, spinner};
use discord_rich_presence::DiscordIpc;

fn build_probe_client() -> HttpClient {
    HttpClient::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .expect("No se pudo crear el cliente HTTP")
}

fn read_rust_env() -> String {
    env::var("rust_env").ok().unwrap_or_else(|| {
        embedded_env::get("rust_env")
            .unwrap_or("production")
            .to_string()
    })
}

pub fn run() {
    let rust_env_pre = read_rust_env();

    if !rust_env_pre.trim().eq_ignore_ascii_case("release") {
        let _ = dotenvy::dotenv();
    }

    set_window_title();

    let current_version = env!("CARGO_PKG_VERSION");
    let rust_env = env::var("rust_env").unwrap_or_else(|_| rust_env_pre.clone());
    banner(current_version, &rust_env);

    let cfg_path = config_path();
    let existed = cfg_path.exists();

    let mut cfg = load_config();

    if existed {
        log_info(format!("📁 Config cargada desde: {}", cfg_path.display()));
    } else {
        log_info(format!(
            "📁 Se ha creado el config.json en: {}",
            cfg_path.display()
        ));
    }

    let discord_client_id = get_required_env("DISCORD_CLIENT_ID");
    let api_key = get_api_key(&mut cfg);
    verify_config(&discord_client_id, &api_key);

    let large_image =
        get_optional_env("DISCORD_LARGE_IMAGE").unwrap_or_else(|| "clashroyale".to_string());

    let small_image_override = get_optional_env("DISCORD_SMALL_IMAGE").unwrap_or_default();
    let arena_base = arena_base_url();

    let debug_arena = env::var("DEBUG_ARENA")
        .ok()
        .map(|v| truthy_env(&v))
        .unwrap_or(false);

    let debug_arena_fetch = env::var("DEBUG_ARENA_FETCH")
        .ok()
        .map(|v| truthy_env(&v))
        .unwrap_or(false);

    let player_poll_ms = parse_u64_env("PLAYER_POLL_MS", 30_000);
    let battle_poll_ms = parse_u64_env("BATTLELOG_POLL_MS", 30_000);
    let tick_ms = parse_u64_env("RPC_TICK_MS", 5_000);

    let player_refresh_retries = parse_u64_env("PLAYER_REFRESH_RETRIES", 3);
    let player_refresh_retry_delay_ms = parse_u64_env("PLAYER_REFRESH_RETRY_DELAY_MS", 1_200);

    let user_agent = format!("{APP_NAME}/{current_version}");
    let http = build_http_client(&user_agent, &api_key);
    let probe_http = build_probe_client();
    let mut last_probe_url = String::new();
    let mut last_probe_at = Instant::now() - Duration::from_secs(3600);

    let player_tag = prompt_player_tag(&http, &mut cfg);

    let pb = spinner("Leyendo tu perfil para obtener tu clan...");
    let player_for_clan = fetch_player(&http, &player_tag).unwrap_or_else(|_| {
        pb.finish_and_clear();
        crate::core::util::die("No pude leer tu perfil para detectar el clan.")
    });
    pb.finish_and_clear();
    prompt_clan_invite_link_if_needed(&mut cfg, &player_for_clan);

    log_info(format!(
        "⏱️ Tick: {}ms • Player poll: {}ms • Battle poll: {}ms (Ctrl+C para cerrar)\n",
        tick_ms, player_poll_ms, battle_poll_ms
    ));

    let mut rpc = connect_rpc_with_retry(&discord_client_id);
    let mut cleared_once = false;

    let mut cached_player: Option<Player> = None;
    let mut last_player_fetch = Instant::now() - Duration::from_secs(3600);

    let mut cached_battle: Option<Option<BattleLogEntry>> = None;
    let mut last_battle_fetch = Instant::now() - Duration::from_secs(3600);

    loop {
        let mut force_player_refresh = false;
        let mut player_fetch_ok_this_tick = false;

        // ---------- PLAYER ----------
        let need_player = cached_player.is_none()
            || last_player_fetch.elapsed() >= Duration::from_millis(player_poll_ms);

        if need_player {
            match fetch_player(&http, &player_tag) {
                Ok(p) => {
                    cached_player = Some(p);
                    last_player_fetch = Instant::now();
                    player_fetch_ok_this_tick = true;
                }
                Err(e) => {
                    log_warn(format!("No pude leer el perfil del jugador ({e})."));
                }
            }
        }

        // ---------- BATTLE ----------
        let need_battle = cached_battle.is_none()
            || last_battle_fetch.elapsed() >= Duration::from_millis(battle_poll_ms);

        if need_battle {
            last_battle_fetch = Instant::now();
            let old_key = cached_battle
                .as_ref()
                .and_then(|opt| opt.as_ref().map(|e| e.battle_time.clone()));

            match fetch_last_battle(&http, &player_tag) {
                Ok(b) => {
                    let new_key = b.as_ref().map(|e| e.battle_time.clone());
                    if new_key != old_key {
                        force_player_refresh = true;
                    }

                    cached_battle = Some(b);
                }
                Err(e) => {
                    log_warn(format!("Battlelog falló ({e})."));
                }
            }
        }

        if force_player_refresh && !player_fetch_ok_this_tick {
            let prev_trophies = cached_player.as_ref().map(|p| p.trophies);

            let mut attempt: u64 = 0;
            loop {
                attempt += 1;

                match fetch_player(&http, &player_tag) {
                    Ok(p) => {
                        let new_trophies = p.trophies;
                        cached_player = Some(p);
                        last_player_fetch = Instant::now();

                        if debug_arena {
                            log_info(format!(
                        "[PLAYER_DEBUG] battle cambió -> refresco el perfil, intento {}/{} ({} -> {})",
                        attempt,
                        player_refresh_retries,
                        prev_trophies.map(|x| x.to_string()).unwrap_or_else(|| "—".to_string()),
                        new_trophies
                    ));
                        }

                        if prev_trophies.map(|t| t != new_trophies).unwrap_or(true) {
                            break;
                        }
                    }
                    Err(e) => {
                        log_warn(format!(
                            "No pude refrescar el perfil tras la actualización ({e})."
                        ));
                    }
                }

                if attempt >= player_refresh_retries {
                    break;
                }

                thread::sleep(Duration::from_millis(player_refresh_retry_delay_ms));
            }
        }

        let Some(player) = cached_player.as_ref() else {
            thread::sleep(Duration::from_millis(tick_ms));
            continue;
        };

        let battle_opt = cached_battle.as_ref().and_then(|x| x.as_ref());
        let arena_dbg = player.arena.as_ref().map(|a| {
            let from_raw = arena_raw_name_to_number(a.raw_name.as_deref());
            let from_name = arena_name_to_number(&a.name);
            let from_id = arena_id_field_to_number(a.id);
            let chosen = arena_id_to_number(a);
            let url = arena_url(a, &arena_base);

            (
                a.id,
                a.name.clone(),
                a.raw_name.clone(),
                from_raw,
                from_name,
                from_id,
                chosen,
                url,
            )
        });

        let small_img = if !small_image_override.trim().is_empty() {
            small_image_override.trim().to_string()
        } else {
            arena_dbg
                .as_ref()
                .and_then(|t| t.7.clone())
                .unwrap_or_else(|| "player".to_string())
        };

        if debug_arena {
            if let Some((id, name, raw, from_raw, from_name, from_id, chosen, url)) =
                arena_dbg.as_ref()
            {
                log_info(format!(
                    "[ARENA_DEBUG] id={} raw={:?} name=\"{}\" -> raw={:?} name={:?} id={:?} chosen={:?} url={:?} small_image=\"{}\"",
                    id, raw, name, from_raw, from_name, from_id, chosen, url, small_img
                ));

                if let (Some(n_name), Some(n_id)) = (*from_name, *from_id) {
                    if n_name != n_id {
                        log_warn(format!(
                            "[ARENA_DEBUG] mismatch: name->{} pero id->{} (se usa chosen={:?})",
                            n_name, n_id, chosen
                        ));
                    }
                }
            } else {
                log_info(format!(
                    "[ARENA_DEBUG] sin arena -> small_image=\"{}\"",
                    small_img
                ));
            }
        }

        if debug_arena_fetch {
            if let Some((_, _, _, _, _, _, _, Some(url))) = arena_dbg.as_ref() {
                let should_probe = last_probe_url != url.as_str()
                    || last_probe_at.elapsed() > Duration::from_secs(300);

                if should_probe {
                    last_probe_url = url.clone();
                    last_probe_at = Instant::now();

                    let (status, ct, cl, err) = probe_image_url(&probe_http, url);
                    log_info(format!(
                        "[ARENA_DEBUG] url=\"{}\" status={} content-type={:?} content-length={:?} err={:?}",
                        url, status, ct, cl, err
                    ));
                }
            }
        }

        let act = build_activity(
            player,
            battle_opt,
            cfg.clan_invite_link.as_deref(),
            &large_image,
            small_img.as_str(),
        );

        if !cleared_once {
            let _ = rpc.clear_activity();
            thread::sleep(Duration::from_millis(200));
            cleared_once = true;
        }

        if let Err(e) = rpc.set_activity(act) {
            log_warn(format!("Falló el SET_ACTIVITY: {e}. Voy a reconectarme..."));
            let _ = rpc.close();
            rpc = connect_rpc_with_retry(&discord_client_id);
            cleared_once = false;
        }

        thread::sleep(Duration::from_millis(tick_ms));
    }
}
