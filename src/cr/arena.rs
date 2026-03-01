use super::models::Arena;
use crate::core::constants::DEFAULT_ARENA_ASSET_BASE_URL;
use crate::core::util::get_optional_env;

pub fn arena_base_url() -> String {
    let raw = get_optional_env("ARENA_ASSET_BASE_URL")
        .unwrap_or_else(|| DEFAULT_ARENA_ASSET_BASE_URL.to_string());

    raw.trim_end_matches('/').to_string()
}

pub fn arena_raw_name_to_number(raw_name: Option<&str>) -> Option<u32> {
    let raw = raw_name?.trim();
    if raw.is_empty() {
        return None;
    }

    let lc = raw.to_lowercase();
    let needle = "arena_l";
    let pos = lc.find(needle)?;
    let start = pos + needle.len();

    let digits: String = lc[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();

    if digits.is_empty() {
        return None;
    }

    let n = digits.parse::<u32>().ok()?;
    (n <= 32).then_some(n)
}

pub fn arena_name_to_number(name: &str) -> Option<u32> {
    let k = name.trim().to_lowercase();

    match k.as_str() {
        "training camp" => Some(0),
        "goblin stadium" => Some(1),
        "bone pit" => Some(2),
        "barbarian bowl" => Some(3),
        "spell valley" => Some(4),
        "builder's workshop" | "builders workshop" => Some(5),
        "p.e.k.k.a's playhouse" | "pekka's playhouse" => Some(6),
        "royal arena" => Some(7),
        "frozen peak" => Some(8),
        "jungle arena" => Some(9),
        "hog mountain" => Some(10),
        "electro valley" => Some(11),
        "spooky town" => Some(12),
        "rascal's hideout" | "rascals hideout" => Some(13),
        "serenity peak" => Some(14),
        "miner's mine" | "miners mine" => Some(15),
        "executioner's kitchen" | "executioners kitchen" => Some(16),
        "royal crypt" => Some(17),
        "silent sanctuary" => Some(18),
        "dragon spa" => Some(19),
        "boot camp" => Some(20),
        "clash fest" => Some(21),
        "pancakes!" | "pankcakes!" => Some(22),
        "valkalla" | "valkella" => Some(23),
        "legendary arena" => Some(24),
        "lumberlove cabin" | "lumberjack's cabin" | "lumberjack cabin" => Some(25),
        "royal road" => Some(26),
        "musketeer street" => Some(27),
        "summit of heroes" => Some(28),
        "magic academy" => Some(29),
        "ultimate clash pit" => Some(30),
        "little prince's tavern" | "little prince’s tavern" => Some(31),
        "spirit square" => Some(32),
        _ => None,
    }
}

pub fn arena_id_field_to_number(arena_id: u64) -> Option<u32> {
    if arena_id >= 54_000_000 {
        let maybe = (arena_id - 54_000_000) as u32;
        if maybe <= 32 {
            return Some(maybe);
        }
    }
    None
}

// 1) rawName -> 'Arena_Lxx'
// 2) name -> por si 'rawName' viene roto
// 3) id -> 54000000 + xx (que es lo que antes usaba ClashRPC para detectar la arena, pero a veces no se actualiza bien, por eso las otras opciones)

pub fn arena_id_to_number(arena: &Arena) -> Option<u32> {
    arena_raw_name_to_number(arena.raw_name.as_deref())
        .or_else(|| arena_name_to_number(&arena.name))
        .or_else(|| arena_id_field_to_number(arena.id))
}

pub fn arena_number_to_es(n: u32) -> Option<&'static str> {
    match n {
        0 => Some("Entrenamiento"),
        1 => Some("Estadio Duende"),
        2 => Some("Foso de Huesos"),
        3 => Some("Coliseo Bárbaro"),
        4 => Some("Valle de Hechizos"),
        5 => Some("Taller del Constructor"),
        6 => Some("Fuerte de la P.E.K.K.A."),
        7 => Some("Arena Real"),
        8 => Some("Pico Helado"),
        9 => Some("Arena Selvática"),
        10 => Some("Montepuerco"),
        11 => Some("Electrovalle"),
        12 => Some("Pueblo Espeluznante"),
        13 => Some("Escondite de los Pillos"),
        14 => Some("Pico Sereno"),
        15 => Some("La Gran Mina"),
        16 => Some("La Cocina del Verdugo"),
        17 => Some("Cripta Real"),
        18 => Some("Santuario del Silencio"),
        19 => Some("Termas de Dragones"),
        20 => Some("Campo de Entrenamiento"),
        21 => Some("Clash Fest"),
        22 => Some("¡TORTITAS!"),
        23 => Some("Valkalla"),
        24 => Some("Arena Legendaria"),
        25 => Some("Cabaña del leñador"),
        26 => Some("Sendero Real"),
        27 => Some("Bulevar de la Mosquetera"),
        28 => Some("Cima heroica"),
        29 => Some("Academia de Magia"),
        30 => Some("Arena suprema de combate"),
        31 => Some("Taberna del principito"),
        32 => Some("Plaza espiritual"),
        _ => None,
    }
}

pub fn arena_url(arena: &Arena, base_url: &str) -> Option<String> {
    let n = arena_id_to_number(arena)?;
    Some(format!("{}/arena{}.png", base_url.trim_end_matches('/'), n))
}
