fn main() {
    println!("Popups torture test starting...");

    kurogane::App::new("popups")
        .chromium_flag("disable-popup-blocking")
        .run_or_exit();
}
