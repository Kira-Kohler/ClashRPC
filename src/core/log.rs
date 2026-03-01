use console::style;

pub fn log_info(msg: impl AsRef<str>) {
    println!("{} {}", style("[INFO]").cyan(), msg.as_ref());
}

pub fn log_ok(msg: impl AsRef<str>) {
    println!("{} {}", style("[OK]").green(), msg.as_ref());
}

pub fn log_warn(msg: impl AsRef<str>) {
    println!("{} {}", style("[WARN]").yellow(), msg.as_ref());
}

pub fn log_err(msg: impl AsRef<str>) {
    eprintln!("{} {}", style("[ERROR]").red(), msg.as_ref());
}
