fn main() {
    if linen_tray::relaunch_from_runtime().expect("runtime launch failed") {
        return;
    }
    let executable = std::env::current_exe().unwrap();
    let settings =
        linen_tray::settings_path(&executable, std::env::var_os("APPDATA").as_deref(), "Probe")
            .unwrap();
    std::fs::write(
        std::env::args_os().nth(1).expect("output path missing"),
        format!(
            "{}\n{}\n{}\n{}",
            std::process::id(),
            executable.display(),
            settings.display(),
            std::fs::read_to_string(&settings).unwrap_or_default()
        ),
    )
    .unwrap();
}
