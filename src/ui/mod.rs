use std::time::Duration;

use console::style;
use figlet_rs::FIGfont;
use indicatif::{ProgressBar, ProgressStyle};

use crate::core::constants::{APP_NAME, CREATOR_NAME, CREATOR_URL, WINDOW_TITLE};

pub fn banner(version: &str, rust_env: &str) {
    let _ = console::Term::stdout().clear_screen();

    if let Ok(font) = FIGfont::standard() {
        if let Some(fig) = font.convert(APP_NAME) {
            println!("{}", style(fig).cyan());
        }
    }

    println!(
        "{}",
        style(format!(
            "v{version} • {CREATOR_NAME} • {CREATOR_URL} • env: {rust_env}"
        ))
        .dim()
    );
    println!();
}

pub fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    let style = ProgressStyle::with_template("{spinner} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner());
    pb.set_style(style);
    pb.enable_steady_tick(Duration::from_millis(90));
    pb.set_message(msg.to_string());
    pb
}

#[cfg(windows)]
mod win_title {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "Kernel32")]
    extern "system" {
        fn SetConsoleTitleW(lpConsoleTitle: *const u16) -> i32;
    }

    pub fn set(title: &str) {
        let wide: Vec<u16> = OsStr::new(title)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let _ = SetConsoleTitleW(wide.as_ptr());
        }
    }
}

#[cfg(not(windows))]
mod win_title {
    pub fn set(_title: &str) {}
}

pub fn set_window_title() {
    win_title::set(WINDOW_TITLE);
    print!("\x1b]0;{}\x07", WINDOW_TITLE);
}
