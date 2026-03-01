use std::{thread, time::Duration};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

use crate::core::constants::{CREATOR_URL, GAME};
use crate::core::log::{log_ok, log_warn};
use crate::cr::arena::{arena_id_to_number, arena_number_to_es};
use crate::cr::battle::battle_state_line;
use crate::cr::models::{BattleLogEntry, Player};
use crate::ui::spinner;

pub fn build_activity<'a>(
    player: &'a Player,
    battle: Option<&'a BattleLogEntry>,
    clan_invite_link: Option<&'a str>,
    large_image: &'a str,
    small_image: &'a str,
) -> activity::Activity<'a> {
    let arena_num = player.arena.as_ref().and_then(|a| arena_id_to_number(a));

    let arena_es = match arena_num.and_then(|n| arena_number_to_es(n)) {
        Some(es) => es.to_string(),
        None => match arena_num {
            Some(n) => format!("Arena {}", n),
            None => "—".to_string(),
        },
    };

    let details = format!(
        "👤{} • ⭐{} • 🏆{}",
        player.name, player.exp_level, player.trophies
    );

    let state = battle_state_line(battle);

    let small_text = match arena_num {
        Some(n) => format!("[{}] {}", n, arena_es),
        None => arena_es.clone(),
    };

    let mut act = activity::Activity::new()
        .name(GAME)
        .details(details)
        .state(state)
        .assets(
            activity::Assets::new()
                .large_image(large_image)
                .large_text(GAME)
                .small_image(small_image)
                .small_text(small_text),
        );

    let mut buttons: Vec<activity::Button> = Vec::new();

    if let Some(url) = clan_invite_link {
        buttons.push(activity::Button::new("🤝 Unirse al clan", url));
    }

    buttons.push(activity::Button::new(
        "Created by: Kira Kohler",
        CREATOR_URL,
    ));

    act = act.buttons(buttons);

    act
}

pub fn connect_rpc_with_retry(discord_client_id: &str) -> DiscordIpcClient {
    loop {
        let mut rpc = DiscordIpcClient::new(discord_client_id);

        let pb = spinner("Conectando a Discord RPC...");
        match rpc.connect() {
            Ok(_) => {
                pb.finish_and_clear();
                let _ = rpc.clear_activity();

                log_ok("Rich Presence conectado a Discord.");
                return rpc;
            }
            Err(e) => {
                pb.finish_and_clear();
                log_warn(format!(
                    "No me pude conectar a Discord RPC: {e}. ¿Tienes Discord abierto? Reintento en 5s..."
                ));
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}
