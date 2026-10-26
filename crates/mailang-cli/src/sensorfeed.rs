//! File-fed sensor backend for the `bme280` pack (`--bme280-feed=<file>`).
//!
//! The pack ships a pure-Mai simulation and declares "cooked" host hooks for
//! real sensors (`bme280_temp_c` / `bme280_humidity_pct` / `bme280_pressure_pa`).
//! On a bench without a wired sensor this backend implements those hooks from
//! a CSV file so the host-override path is exercised end to end and a run can
//! consume *real recorded* readings instead of the pack's synthetic constants:
//!
//! ```text
//! # temp_c,humidity_pct,pressure_pa   (lines starting with # are skipped)
//! 21.4,48.2,101325.0
//! 21.5,48.1,101324.0
//! ```
//!
//! Readings are served in file order and wrap around at the end. This is a
//! **replay**, not a sensor: it is honestly labelled as such and is distinct
//! from the pack's sim in that the numbers come from the file, not from the
//! pack. A real sensor needs an ESP-IDF / MCU host implementing the same
//! three hook names over I2C.

use mailang_core::{bytecode::Value, MailangInterpreter};
use std::cell::Cell;
use std::rc::Rc;

/// Parse the feed file; returns one `[temp, hum, press]` row per data line.
fn parse_feed(text: &str) -> Result<Vec<[f64; 3]>, String> {
    let mut rows = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        if parts.len() != 3 {
            return Err(format!(
                "bme280 feed line {}: expected 'temp_c,humidity_pct,pressure_pa', got {:?}",
                n + 1,
                line
            ));
        }
        let mut row = [0.0f64; 3];
        for (i, p) in parts.iter().enumerate() {
            row[i] = p
                .parse::<f64>()
                .map_err(|e| format!("bme280 feed line {}: bad number '{}': {}", n + 1, p, e))?;
        }
        rows.push(row);
    }
    if rows.is_empty() {
        return Err("bme280 feed: no data rows".into());
    }
    Ok(rows)
}

/// Register the three cooked hooks. Each hook keeps its own cursor and walks
/// the feed in file order, wrapping at the end — so a loop over
/// `bme280_temp_c()` replays the recorded temperature series.
pub fn install(interp: &mut MailangInterpreter, path: &std::path::Path) -> Result<(), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("bme280 feed: cannot read '{}': {}", path.display(), e))?;
    let rows = Rc::new(parse_feed(&text)?);

    for (slot, name) in [
        (0usize, "bme280_temp_c"),
        (1, "bme280_humidity_pct"),
        (2, "bme280_pressure_pa"),
    ] {
        let rows = rows.clone();
        let idx = Rc::new(Cell::new(0usize));
        interp.register_host_fn(name, move |_args| {
            let i = idx.get() % rows.len();
            idx.set(idx.get() + 1);
            Ok(Value::Float(rows[i][slot]))
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rows_and_skips_comments() {
        let rows = parse_feed("# comment\n21.4,48.2,101325.0\n\n22.0,50,101300\n").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], [21.4, 48.2, 101325.0]);
        assert_eq!(rows[1], [22.0, 50.0, 101300.0]);
    }

    #[test]
    fn rejects_bad_rows() {
        assert!(parse_feed("1,2\n").is_err());
        assert!(parse_feed("a,b,c\n").is_err());
        assert!(parse_feed("# only a comment\n").is_err());
    }

    #[test]
    fn hooks_cycle_through_the_feed() {
        let dir = std::env::temp_dir().join(format!("mailang_feed_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("feed.csv");
        std::fs::write(&file, "21.4,48.2,101325.0\n22.5,49.0,101300.0\n").unwrap();

        let mut interp = MailangInterpreter::new();
        install(&mut interp, &file).unwrap();
        assert_eq!(interp.eval("bme280_temp_c()").unwrap(), "21.4");
        assert_eq!(interp.eval("bme280_temp_c()").unwrap(), "22.5");
        assert_eq!(interp.eval("bme280_temp_c()").unwrap(), "21.4"); // wraps
        assert_eq!(interp.eval("bme280_humidity_pct()").unwrap(), "48.2");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
