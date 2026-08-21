use std::{thread, time::Duration};

use ts_platform::MacOsMonitor;

fn main() {
    let monitor = MacOsMonitor::new();
    loop {
        println!(
            "idle: {} ms   mono: {} ms",
            monitor.idle_ms(),
            monitor.mono_ms()
        );
        thread::sleep(Duration::from_secs(1));
    }
}
