//! Forward the linker arguments esp-idf-sys produced (EMBUILD_LINK_ARGS
//! metadata: the real cross-gcc path for ldproxy, IDF linker scripts and
//! component archives) to the final binary, plus the ESP_IDF cfgs.

use embuild::build::{CfgArgs, LinkArgs};

fn main() {
    // Fail gracefully when built outside the esp container (plain `cargo
    // check` on the host would otherwise report a confusing deep error).
    if std::env::var("IDF_PATH").is_err() {
        println!(
            "cargo:warning=IDF_PATH is not set — build via scripts/esp32-qemu.sh or the \
             espressif/idf-rust container with ESP-IDF available"
        );
        return;
    }
    LinkArgs::output_propagated("ESP_IDF").unwrap();
    CfgArgs::output_propagated("ESP_IDF").unwrap();
}
