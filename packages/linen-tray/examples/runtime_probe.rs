fn main() {
    if linen_tray::relaunch_from_runtime().expect("runtime launch failed") {
        return;
    }
    std::fs::write(
        std::env::args_os().nth(1).expect("output path missing"),
        format!(
            "{}\n{}",
            std::process::id(),
            std::env::current_exe().unwrap().display()
        ),
    )
    .unwrap();
}
