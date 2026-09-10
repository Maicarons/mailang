//! Time built-in functions

use mailang_bytecode::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn builtin_time_now(_args: &[Value]) -> Result<Value, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    Ok(Value::Int(duration.as_millis() as i64))
}

pub fn builtin_time_now_secs(_args: &[Value]) -> Result<Value, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    Ok(Value::Int(duration.as_secs() as i64))
}

pub fn builtin_time_year(_args: &[Value]) -> Result<Value, String> {
    let secs = get_epoch_secs()?;
    let (year, _, _) = epoch_to_ymd(secs);
    Ok(Value::Int(year))
}

pub fn builtin_time_month(_args: &[Value]) -> Result<Value, String> {
    let secs = get_epoch_secs()?;
    let (_, month, _) = epoch_to_ymd(secs);
    Ok(Value::Int(month))
}

pub fn builtin_time_day(_args: &[Value]) -> Result<Value, String> {
    let secs = get_epoch_secs()?;
    let (_, _, day) = epoch_to_ymd(secs);
    Ok(Value::Int(day))
}

pub fn builtin_time_hour(_args: &[Value]) -> Result<Value, String> {
    let secs = get_epoch_secs()?;
    let (_, h, _, _) = epoch_to_hms(secs);
    Ok(Value::Int(h))
}

pub fn builtin_time_minute(_args: &[Value]) -> Result<Value, String> {
    let secs = get_epoch_secs()?;
    let (_, _, m, _) = epoch_to_hms(secs);
    Ok(Value::Int(m))
}

pub fn builtin_time_second(_args: &[Value]) -> Result<Value, String> {
    let secs = get_epoch_secs()?;
    let (_, _, _, s) = epoch_to_hms(secs);
    Ok(Value::Int(s))
}

pub fn builtin_time_date(_args: &[Value]) -> Result<Value, String> {
    let secs = get_epoch_secs()?;
    let (year, month, day) = epoch_to_ymd(secs);
    Ok(Value::Str(format!("{:04}-{:02}-{:02}", year, month, day)))
}

pub fn builtin_time_datetime(_args: &[Value]) -> Result<Value, String> {
    let secs = get_epoch_secs()?;
    let (year, month, day) = epoch_to_ymd(secs);
    let (_, hour, minute, second) = epoch_to_hms(secs);
    Ok(Value::Str(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    )))
}

pub fn builtin_time_elapsed(args: &[Value]) -> Result<Value, String> {
    let start = match &args[0] {
        Value::Int(n) => *n,
        _ => return Err("elapsed expects an integer timestamp".to_string()),
    };
    let now = builtin_time_now(&[])?;
    match now {
        Value::Int(now_ms) => Ok(Value::Int(now_ms - start)),
        _ => Err("Failed to get current time".to_string()),
    }
}

pub fn builtin_time_sleep(args: &[Value]) -> Result<Value, String> {
    let ms = match &args[0] {
        Value::Int(n) => *n as u64,
        Value::Float(n) => *n as u64,
        _ => return Err("sleep expects a number".to_string()),
    };
    std::thread::sleep(std::time::Duration::from_millis(ms));
    Ok(Value::Null)
}

fn get_epoch_secs() -> Result<i64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    Ok(duration.as_secs() as i64)
}

/// Convert epoch seconds to (year, month, day) - UTC
fn epoch_to_ymd(epoch: i64) -> (i64, i64, i64) {
    let days = epoch / 86400;
    let mut y = 1970;
    let mut remaining_days = days;

    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }

    let leap = is_leap_year(y);
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 1;
    for &days_in_month in &month_days {
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        m += 1;
    }

    (y, m, remaining_days + 1)
}

/// Convert epoch seconds to (_, hour, minute, second) - UTC
fn epoch_to_hms(epoch: i64) -> (i64, i64, i64, i64) {
    let secs_in_day = epoch % 86400;
    let hour = secs_in_day / 3600;
    let minute = (secs_in_day % 3600) / 60;
    let second = secs_in_day % 60;
    (0, hour, minute, second)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
