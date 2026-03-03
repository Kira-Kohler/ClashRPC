use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::core::constants::APP_NAME;
use crate::core::log::{log_info, log_warn};
use crate::core::util::{get_optional_env, parse_u64_env, truthy_env};

const REPO_OWNER: &str = "Kira-Kohler";
const REPO_NAME: &str = "ClashRPC";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PendingUpdate {
    version: String,
    exe_path: String,
}

fn exe_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

#[cfg(target_os = "windows")]
fn strip_windows_verbatim_prefix(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();

    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{}", rest);
    }

    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return rest.to_string();
    }

    s
}

#[cfg(not(target_os = "windows"))]
fn strip_windows_verbatim_prefix(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn updates_dir() -> PathBuf {
    if let Some(custom) = get_optional_env("CLASHRPC_UPDATES_DIR") {
        let custom = custom.trim();
        if !custom.is_empty() {
            return PathBuf::from(custom);
        }
    }
    std::env::temp_dir().join(APP_NAME).join("updates")
}

fn pending_manifest_path() -> PathBuf {
    updates_dir().join("pending.json")
}

fn normalize_version_tag(tag: &str) -> Option<Version> {
    let t = tag.trim().trim_start_matches('v').trim_start_matches('V');
    Version::parse(t).ok()
}

fn updates_disabled() -> bool {
    let v = get_optional_env("CLASHRPC_DISABLE_AUTO_UPDATE").unwrap_or_default();
    !v.trim().is_empty() && truthy_env(&v)
}

fn github_client(user_agent: &str) -> Client {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(user_agent)
        .build()
        .expect("No se pudo crear el cliente HTTP (updates)")
}

fn fetch_latest_release(client: &Client) -> Result<GithubRelease, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );

    let resp = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| format!("No pude consultar releases: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub respondió {}", resp.status()));
    }

    resp.json::<GithubRelease>()
        .map_err(|e| format!("No pude parsear JSON de GitHub: {e}"))
}

fn pick_asset_for_windows(assets: &[GithubAsset]) -> Option<&GithubAsset> {
    let mut best_zip_windows: Option<&GithubAsset> = None;
    let mut any_zip: Option<&GithubAsset> = None;
    let mut any_exe: Option<&GithubAsset> = None;

    for a in assets {
        let n = a.name.to_ascii_lowercase();

        if n.ends_with(".zip") {
            if n.contains("windows") || n.contains("win") {
                best_zip_windows = Some(a);
            }
            if any_zip.is_none() {
                any_zip = Some(a);
            }
        }

        if n.ends_with(".exe") {
            if any_exe.is_none() {
                any_exe = Some(a);
            }
        }
    }

    best_zip_windows.or(any_zip).or(any_exe)
}

fn download_to_file(client: &Client, url: &str, out: &Path) -> Result<(), String> {
    let mut resp = client
        .get(url)
        .send()
        .map_err(|e| format!("Descarga fallida: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Descarga fallida: HTTP {}", resp.status()));
    }

    let mut f = fs::File::create(out).map_err(|e| {
        format!(
            "No pude crear el archivo de descarga ({}): {e}",
            out.display()
        )
    })?;

    resp.copy_to(&mut f)
        .map_err(|e| format!("No pude guardar la descarga: {e}"))?;

    Ok(())
}

fn extract_exe_from_zip(zip_path: &Path, version: &Version) -> Result<PathBuf, String> {
    let f = fs::File::open(zip_path)
        .map_err(|e| format!("No pude abrir el ZIP ({}): {e}", zip_path.display()))?;

    let mut archive =
        zip::ZipArchive::new(f).map_err(|e| format!("ZIP corrupto o inválido: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("No pude leer la entrada del ZIP: {e}"))?;

        let name = file.name().to_string();
        let lower = name.to_ascii_lowercase();

        if lower.ends_with(".exe") && lower.contains("clashrpc") {
            let out = updates_dir().join(format!("ClashRPC-{}.exe", version));
            let mut out_f = fs::File::create(&out)
                .map_err(|e| format!("No pude crear el .exe extraído: {e}"))?;

            io::copy(&mut file, &mut out_f)
                .map_err(|e| format!("No pude extraer el .exe del ZIP: {e}"))?;
            return Ok(out);
        }
    }

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("No pude leer la entrada del ZIP: {e}"))?;

        let name = file.name().to_string();
        let lower = name.to_ascii_lowercase();

        if lower.ends_with(".exe") {
            let out = updates_dir().join(format!("ClashRPC-{}.exe", version));
            let mut out_f = fs::File::create(&out)
                .map_err(|e| format!("No pude crear el .exe extraído: {e}"))?;

            io::copy(&mut file, &mut out_f)
                .map_err(|e| format!("No pude extraer el .exe del ZIP: {e}"))?;
            return Ok(out);
        }
    }

    Err("El ZIP no trae ningún .exe dentro".to_string())
}

fn write_pending(version: &Version, exe_path: &Path) -> Result<(), String> {
    let pending = PendingUpdate {
        version: version.to_string(),
        exe_path: strip_windows_verbatim_prefix(exe_path),
    };

    fs::create_dir_all(updates_dir())
        .map_err(|e| format!("No pude crear la carpeta de updates: {e}"))?;

    let path = pending_manifest_path();
    let s = serde_json::to_string_pretty(&pending)
        .map_err(|e| format!("No pude serializar pending.json: {e}"))?;

    fs::write(&path, s)
        .map_err(|e| format!("No pude escribir pending.json ({}): {e}", path.display()))?;
    Ok(())
}

fn read_pending() -> Option<PendingUpdate> {
    let path = pending_manifest_path();
    let s = fs::read_to_string(path).ok()?;
    serde_json::from_str::<PendingUpdate>(&s).ok()
}

fn version_from_filename(name: &str) -> Option<Version> {
    let lower = name.to_ascii_lowercase();

    if lower.starts_with("clashrpc-") && lower.ends_with(".exe") {
        let v = &name["ClashRPC-".len()..name.len() - ".exe".len()];
        return normalize_version_tag(v);
    }

    if lower.starts_with("download-")
        && (lower.ends_with(".zip") || lower.ends_with(".exe") || lower.ends_with(".bin"))
    {
        let ext_len = if lower.ends_with(".zip") {
            4
        } else if lower.ends_with(".exe") {
            4
        } else {
            4
        };

        let v = &name["download-".len()..name.len() - ext_len];
        return normalize_version_tag(v);
    }

    None
}

fn cleanup_updates_dir_keep_latest() -> Result<(), String> {
    use std::collections::HashSet;

    let dir = updates_dir();
    if !dir.exists() {
        return Ok(());
    }

    let mut keep_ver: Option<Version> =
        read_pending().and_then(|p| normalize_version_tag(&p.version));

    for entry in fs::read_dir(&dir).map_err(|e| format!("No pude leer el updates dir: {e}"))? {
        let entry = entry.map_err(|e| format!("No pude leer entrada del dir: {e}"))?;
        if !entry
            .file_type()
            .map_err(|e| format!("No pude leer el tipo de archivo: {e}"))?
            .is_file()
        {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(v) = version_from_filename(&name) {
            keep_ver = match keep_ver {
                Some(cur) => {
                    if v > cur {
                        Some(v)
                    } else {
                        Some(cur)
                    }
                }
                None => Some(v),
            };
        }
    }

    let Some(ver) = keep_ver else {
        return Ok(());
    };

    let ver_s = ver.to_string();
    let mut keep: HashSet<PathBuf> = HashSet::new();

    keep.insert(pending_manifest_path());
    keep.insert(dir.join("apply_update.bat"));
    keep.insert(dir.join(format!("ClashRPC-{}.exe", ver_s)));
    keep.insert(dir.join(format!("download-{}.zip", ver_s)));
    keep.insert(dir.join(format!("download-{}.exe", ver_s)));
    keep.insert(dir.join(format!("download-{}.bin", ver_s)));

    if let Some(p) = read_pending() {
        keep.insert(PathBuf::from(p.exe_path));
    }

    for entry in fs::read_dir(&dir).map_err(|e| format!("No pude leer updates dir: {e}"))? {
        let entry = entry.map_err(|e| format!("No pude leer la entrada del dir: {e}"))?;
        let path = entry.path();

        let ft = entry
            .file_type()
            .map_err(|e| format!("No pude leer el tipo de archivo: {e}"))?;

        if !ft.is_file() {
            continue;
        }

        if keep.contains(&path) {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name.contains(&ver_s) {
            continue;
        }

        let _ = fs::remove_file(&path);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn spawn_apply_script(new_exe: &Path, dest_exe: &Path, manifest: &Path) -> Result<(), String> {
    fs::create_dir_all(updates_dir())
        .map_err(|e| format!("No pude crear la carpeta de updates: {e}"))?;

    let bat_path = updates_dir().join("apply_update.bat");

    let bat = "@echo off\r\n\
        setlocal\r\n\
        set \"NEW=%~1\"\r\n\
        set \"DEST=%~2\"\r\n\
        set \"MAN=%~3\"\r\n\
        \r\n\
        ping 127.0.0.1 -n 2 > nul\r\n\
        \r\n\
        if not exist \"%NEW%\" exit /b 1\r\n\
        \r\n\
        if exist \"%DEST%\" (\r\n\
        move /Y \"%DEST%\" \"%DEST%.old\" >nul 2>&1\r\n\
        )\r\n\
        \r\n\
        move /Y \"%NEW%\" \"%DEST%\" >nul 2>&1\r\n\
        \r\n\
        if not exist \"%DEST%\" (\r\n\
        if exist \"%DEST%.old\" move /Y \"%DEST%.old\" \"%DEST%\" >nul 2>&1\r\n\
        exit /b 1\r\n\
        )\r\n\
        \r\n\
        if exist \"%DEST%.old\" del \"%DEST%.old\" >nul 2>&1\r\n\
        if exist \"%MAN%\" del \"%MAN%\" >nul 2>&1\r\n\
        \r\n\
        start \"\" \"%DEST%\"\r\n\
        del \"%~f0\" >nul 2>&1\r\n";

    fs::write(&bat_path, bat).map_err(|e| format!("No pude escribir apply_update.bat: {e}"))?;

    let bat_s = strip_windows_verbatim_prefix(&bat_path);
    let new_s = strip_windows_verbatim_prefix(new_exe);
    let dest_s = strip_windows_verbatim_prefix(dest_exe);
    let man_s = strip_windows_verbatim_prefix(manifest);

    Command::new("cmd")
        .arg("/C")
        .arg("call")
        .arg(&bat_s)
        .arg(&new_s)
        .arg(&dest_s)
        .arg(&man_s)
        .spawn()
        .map_err(|e| format!("No pude lanzar el updater (.bat): {e}"))?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn spawn_apply_script(_new_exe: &Path, _dest_exe: &Path, _manifest: &Path) -> Result<(), String> {
    Err("El auto-update solo está implementado para Windows en este proyecto".to_string())
}

pub fn try_apply_pending_update(current_version: &str) {
    if updates_disabled() {
        return;
    }

    let Some(pending) = read_pending() else {
        return;
    };

    let Some(cur) = normalize_version_tag(current_version) else {
        return;
    };

    let Some(pend_ver) = normalize_version_tag(&pending.version) else {
        return;
    };

    if pend_ver <= cur {
        let _ = fs::remove_file(pending_manifest_path());
        return;
    }

    let new_exe = PathBuf::from(pending.exe_path);
    if !new_exe.exists() {
        log_warn("Tenía una actualización pendiente, pero el archivo no existe, mas tarde lo volveré a comprobar.");
        let _ = fs::remove_file(pending_manifest_path());
        return;
    }

    let Some(dest) = exe_path() else {
        return;
    };

    log_info(format!(
        "🆕 Tengo una actualización pendiente ({}). La aplico y reinicio...",
        pend_ver
    ));

    match spawn_apply_script(&new_exe, &dest, &pending_manifest_path()) {
        Ok(_) => {
            std::process::exit(0);
        }
        Err(e) => {
            log_warn(format!(
                "No pude aplicar la actualización automáticamente: {e}"
            ));
        }
    }
}

fn check_once_and_download(current_version: &str) -> Result<(), String> {
    if updates_disabled() {
        return Ok(());
    }

    let Some(cur) = normalize_version_tag(current_version) else {
        return Ok(());
    };

    if let Some(p) = read_pending() {
        if let Some(pv) = normalize_version_tag(&p.version) {
            if pv > cur {
                return Ok(());
            }
        }
    }

    let user_agent = format!("{APP_NAME}/{current_version}");
    let client = github_client(&user_agent);
    let rel = fetch_latest_release(&client)?;

    if rel.prerelease {
        return Ok(());
    }

    let Some(latest) = normalize_version_tag(&rel.tag_name) else {
        return Ok(());
    };

    if latest <= cur {
        return Ok(());
    }

    let Some(asset) = pick_asset_for_windows(&rel.assets) else {
        return Err("No encontré assets en la release".to_string());
    };

    fs::create_dir_all(updates_dir())
        .map_err(|e| format!("No pude crear la carpeta de updates: {e}"))?;

    let name_lower = asset.name.to_ascii_lowercase();
    let ext = if name_lower.ends_with(".zip") {
        "zip"
    } else if name_lower.ends_with(".exe") {
        "exe"
    } else {
        "bin"
    };

    let downloaded = updates_dir().join(format!("download-{}.{}", latest, ext));

    log_info(format!(
        "🧩 Actu encontrada: {} (tú tienes la {}). Descargando nueva versión...",
        latest, cur
    ));

    download_to_file(&client, &asset.browser_download_url, &downloaded)?;

    let exe_ready = if ext == "zip" {
        extract_exe_from_zip(&downloaded, &latest)?
    } else {
        let out = updates_dir().join(format!("ClashRPC-{}.exe", latest));
        let _ = fs::copy(&downloaded, &out);
        out
    };

    write_pending(&latest, &exe_ready)?;

    if let Err(e) = cleanup_updates_dir_keep_latest() {
        log_warn(format!("Cleanup updates: {e}"));
    }

    log_info(format!(
        "✅ Actualización descargada ({}). Se aplicará al reiniciar ClashRPC.",
        latest
    ));

    Ok(())
}

pub fn check_startup_update(current_version: &str) {
    if let Err(e) = cleanup_updates_dir_keep_latest() {
        log_warn(format!("Cleanup updates: {e}"));
    }

    if let Err(e) = check_once_and_download(current_version) {
        log_warn(format!("Auto-update (startup): {e}"));
    }
}

pub fn start_auto_update_thread(current_version: String) {
    if updates_disabled() {
        return;
    }

    let interval_mins = parse_u64_env("AUTO_UPDATE_INTERVAL_MINS", 30);
    let interval = Duration::from_secs(interval_mins.saturating_mul(60));
    if interval.as_secs() == 0 {
        return;
    }

    thread::spawn(move || {
        thread::sleep(interval);

        loop {
            if let Err(e) = check_once_and_download(&current_version) {
                log_warn(format!("Auto-update: {e}"));
            }
            thread::sleep(interval);
        }
    });
}
