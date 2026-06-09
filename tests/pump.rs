use std::time::Duration;

use kurogane::App;

fn main() {
    let runtime = App::url("https://example.com")
        .start()
        .expect("Kurogane failed to initialize");

    let tick = Duration::from_millis(16);

    while !runtime.should_shutdown() {
        runtime.pump();
        std::thread::sleep(tick);
    }

    runtime.shutdown();
}
