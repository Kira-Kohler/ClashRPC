use chrono::{DateTime, Utc};

use super::models::{BattleLogEntry, BattleSide};

pub fn parse_battle_time_to_utc(battle_time: &str) -> Option<DateTime<Utc>> {
    let s = battle_time.trim();
    let m = s.chars().take(15).collect::<String>();
    if m.len() < 15 {
        return None;
    }

    let y = &s[0..4];
    let mo = &s[4..6];
    let d = &s[6..8];
    let h = &s[9..11];
    let mi = &s[11..13];
    let sec = &s[13..15];

    let iso = format!("{y}-{mo}-{d}T{h}:{mi}:{sec}.000Z");
    chrono::DateTime::parse_from_rfc3339(&iso)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn format_ago(dt: Option<DateTime<Utc>>) -> String {
    let Some(dt) = dt else {
        return "hace —".to_string();
    };

    let diff = Utc::now().signed_duration_since(dt);
    let mins = diff.num_minutes();

    if mins < 60 {
        return format!("hace {mins} min");
    }

    let hours = diff.num_hours();
    if hours < 24 {
        return format!("hace {hours} h");
    }

    let days = diff.num_days();
    format!("hace {days} d")
}

pub fn max_crowns(arr: &[BattleSide]) -> u32 {
    arr.iter().filter_map(|p| p.crowns).max().unwrap_or(0)
}

pub fn result_text(team: u32, opp: u32) -> &'static str {
    if team > opp {
        "✅ Victoria"
    } else if team < opp {
        "❌ Derrota"
    } else {
        "🤝 Empate"
    }
}

pub fn translate_mode(mode: &str) -> &str {
    let m = mode.trim().to_lowercase();

    if m.contains("ladder") {
        return "Camino de trofeos";
    }

    if m.contains("ranked") || m.contains("path of legends") || m.contains("legends") {
        return "Ranked";
    }

    if m.contains("2v2") {
        return "2c2";
    }

    if m.contains("tournament") {
        return "Torneo";
    }

    if m.contains("friendly") {
        return "Amistosa";
    }

    if m.contains("clan war") || m.contains("clanwar") {
        return "Guerra de clanes";
    }

    if m.contains("grand challenge") {
        return "Gran desafío";
    }

    if m.contains("classic challenge") {
        return "Desafío clásico";
    }

    if m.contains("challenge") {
        return "Desafío";
    }

    mode
}

pub fn battle_state_line(battle: Option<&BattleLogEntry>) -> String {
    let mut state = "Última partida: —".to_string();

    if let Some(b) = battle {
        let team = max_crowns(&b.team);
        let opp = max_crowns(&b.opponent);
        let score = format!("{team}-{opp}");

        let when = format_ago(parse_battle_time_to_utc(&b.battle_time));
        let raw_mode = b
            .game_mode
            .as_ref()
            .and_then(|m| m.name.as_deref())
            .or(b.r#type.as_deref())
            .unwrap_or("Partida");

        let mode = translate_mode(raw_mode);
        let res = result_text(team, opp);

        state = format!("{res} {score} • {mode} • {when}");
    }

    state
}
