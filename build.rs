use std::{env, fs, path::Path};

fn escape_rust_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn parse_dotenv(path: &Path) -> Vec<(String, String)> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut out = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((k, v)) = line.split_once('=') else {
            continue;
        };

        let key = k.trim().to_string();
        let mut val = v.trim().to_string();
        if (val.starts_with('"') && val.ends_with('"'))
            || (val.starts_with('\'') && val.ends_with('\''))
        {
            if val.len() >= 2 {
                val = val[1..val.len() - 1].to_string();
            }
        }

        out.push((key, val));
    }

    out
}

fn pack_win_version(major: u16, minor: u16, patch: u16, build: u16) -> u64 {
    ((major as u64) << 48) | ((minor as u64) << 32) | ((patch as u64) << 16) | (build as u64)
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let profile = env::var("PROFILE").unwrap_or_default();
    let is_release = profile == "release";

    if !is_release {
        println!("cargo:rerun-if-changed=.env");
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR missing");
    let out_file = Path::new(&out_dir).join("embedded_env.rs");

    println!("cargo:rerun-if-changed=.env");

    let pairs = parse_dotenv(Path::new(".env"));

    let wanted_release = [
        "DISCORD_CLIENT_ID",
        "DISCORD_LARGE_IMAGE",
        "DISCORD_SMALL_IMAGE",
        "RUST_ENV",
        "ARENA_ASSET_BASE_URL",
        "CLAN_INVITE_LINK",
        "CLASH_ROYALE_PLAYER_TAG",
    ];

    let wanted_dev = [
        "CLASH_ROYALE_API_KEY",
        "DISCORD_CLIENT_ID",
        "DISCORD_LARGE_IMAGE",
        "DISCORD_SMALL_IMAGE",
        "RUST_ENV",
        "ARENA_ASSET_BASE_URL",
        "CLAN_INVITE_LINK",
        "CLASH_ROYALE_PLAYER_TAG",
    ];

    let wanted = if is_release {
        &wanted_release[..]
    } else {
        &wanted_dev[..]
    };

    let mut match_arms = String::new();
    for (k, v) in pairs {
        if wanted.contains(&k.as_str()) {
            let v = escape_rust_string(&v);
            match_arms.push_str(&format!("        \"{k}\" => Some(\"{v}\"),\n"));
        }
    }

    let generated = format!(
        "\n\
    pub fn get(key: &str) -> Option<&'static str> {{\n\
        match key {{\n\
    {match_arms}\
            _ => None,\n\
        }}\n\
    }}\n"
    );

    fs::write(&out_file, generated).expect("No se pudo escribir embedded_env.rs");

    if env::var_os("CARGO_CFG_TARGET_OS").as_deref() == Some(std::ffi::OsStr::new("windows")) {
        let mut res = winres::WindowsResource::new();

        res.set_icon("assets/ClashRPC.ico");

        res.set(
            "FileDescription",
            "ClashRPC - Un RPC de Discord para Clash Royale - hecho por Kira Kohler",
        );
        res.set("ProductName", "ClashRPC");
        res.set("CompanyName", "Kira Kohler");
        res.set("OriginalFilename", "ClashRPC.exe");
        res.set("InternalName", "ClashRPC");
        res.set(
            "LegalCopyright",
            "Copyright © 2026 Kira Kohler. Todos los derechos reservados.",
        );

        let v = pack_win_version(1, 1, 1, 0);
        res.set_version_info(winres::VersionInfo::FILEVERSION, v);
        res.set_version_info(winres::VersionInfo::PRODUCTVERSION, v);

        res.compile()
            .expect("No se pudo compilar el recurso del icono");
    }
}
