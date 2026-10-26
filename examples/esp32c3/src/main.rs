//! ESP32-C3 end-to-end: MaìLang (parser → compiler → register VM) runs on the
//! MCU; `gpio_write` / `delay_ms` are host functions backed by real ESP-IDF
//! calls. The blink logic is MaìLang source, embedded as a string so the demo
//! needs no filesystem.
//!
//! Serial output (115200): ON/OFF cycles plus a final "done" line — this is
//! the evidence captured in docs/v0.4/guide/esp32-blink.md.

use esp_idf_sys::{gpio_config, gpio_config_t, gpio_set_level, vTaskDelay};
use mailang_core::{bytecode::Value, MailangInterpreter};
const BLINK_GPIO: i32 = 2;

const BLINK_SRC: &str = r#"
let led = 2
var count = 0
println("iot_blink: LED pin {led}")
while count < 5 {
    gpio_write(led, true)
    println("ON  #{count}")
    delay_ms(500)
    gpio_write(led, false)
    println("OFF #{count}")
    delay_ms(500)
    count = count + 1
}
gpio_write(led, false)
println("iot_blink: done after {count} cycles")
"#;

fn arg_int(args: &[Value], i: usize) -> i32 {
    match args.get(i) {
        Some(Value::Int(n)) => *n as i32,
        Some(Value::Float(f)) => *f as i32,
        _ => 0,
    }
}

fn main() {
    // The interpreter's parser/compiler/VM recursion blows the ~8 KiB default
    // main-task stack (heap-corrupting it on the way). Run everything on a
    // dedicated 64 KiB worker instead.
    let worker = std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(run_demo)
        .expect("spawn interpreter task");
    worker.join().expect("interpreter task panicked");
}

fn run_demo() {
    // Configure the LED pin as a plain output.
    let io_conf = gpio_config_t {
        pin_bit_mask: 1u64 << BLINK_GPIO,
        mode: 2, // GPIO_MODE_OUTPUT
        pull_up_en: 0,
        pull_down_en: 0,
        intr_type: 0,
    };
    unsafe { gpio_config(&io_conf) };

    println!("mailang-esp32c3: booting interpreter on the MCU");

    let mut interp = MailangInterpreter::new();

    interp.register_host_fn("gpio_write", |args| {
        let pin = arg_int(args, 0);
        let on = matches!(args.get(1), Some(Value::Bool(true)) | Some(Value::Int(1)));
        unsafe { gpio_set_level(pin as i32, on as u32) };
        Ok(Value::Null)
    });

    interp.register_host_fn("delay_ms", |args| {
        let ms = arg_int(args, 0).max(0) as u32;
        unsafe { vTaskDelay(ms / 10) }; // 10 ms per tick (configTICK_RATE_HZ = 100)
        Ok(Value::Null)
    });

    match interp.eval(BLINK_SRC) {
        Ok(out) => {
            if !out.is_empty() && out != "null" {
                println!("{}", out);
            }
        }
        Err(e) => println!("mailang-esp32c3: eval failed: {}", e),
    }

    // Keep the task alive so the serial monitor does not see a reboot loop.
    loop {
        unsafe { vTaskDelay(100) }; // 1 s
    }
}
