use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Arena {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub raw_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Clan {
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub name: String,
    pub tag: String,
    pub exp_level: u32,
    pub trophies: u32,
    pub best_trophies: u32,
    #[serde(default)]
    pub arena: Option<Arena>,

    #[serde(default)]
    pub clan: Option<Clan>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BattleSide {
    #[serde(default)]
    pub crowns: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GameMode {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BattleLogEntry {
    pub battle_time: String,
    #[serde(default)]
    pub team: Vec<BattleSide>,
    #[serde(default)]
    pub opponent: Vec<BattleSide>,
    #[serde(default)]
    pub game_mode: Option<GameMode>,
    #[serde(default, rename = "type")]
    pub r#type: Option<String>,
}
