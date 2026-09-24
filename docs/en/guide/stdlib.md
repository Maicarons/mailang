# Standard Library

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [stdlib Source](https://github.com/Maicarons/mailang/tree/master/crates/mailang-stdlib)

## Overview

MaìLang's standard library is exposed as global builtins and built-in methods. Only APIs that exist in the current runtime are documented below.

## Input / Output

```
println("Hello!")
print("Enter: ")
let name = input("Name: ")
```

## File I/O (globals)

```
let content = read_file("config.json")
write_file("output.txt", "Hello, World!")
```

There is no `io.read_file` / `io.write_file` / `io.append_file` / `io.exists` / `io.remove_file`. File APIs are the global `read_file` and `write_file` only.

## Math (globals)

```
abs(-5)            // 5
sqrt(16.0)         // 4.0
sin(0.0)           // 0.0
cos(0.0)           // 1.0
floor(3.7)         // 3
ceil(3.2)          // 4
round(3.5)         // 4
min(1, 2, 3)       // 1
max(1, 2, 3)       // 3
```

There is no `math.` namespace and no `random` / `pow` / `log` / inverse trig helpers yet.

## String methods

```
let s = "Hello, MaìLang!"

s.len                    // 15 (property, Unicode chars)
s.trim()
s.to_upper()             // alias: to_uppercase
s.to_lower()             // alias: to_lowercase
s.starts_with("Hello")   // true
s.ends_with("!")         // true
s.contains("Maì")        // true
s.split(",")             // → array
s.replace("World", "MaìLang")
s.repeat(3)
```

## Array methods

```
var arr = [3, 1, 4]

arr.len              // 3 (property)
arr.push(5)
arr.pop()
arr.insert(0, 0)
arr.contains(4)
arr.join(",")
arr.reverse()
arr.clear()
```

## Map methods

```
var map = {"name": "MaìLang"}

map["name"]
map["version"] = "0.2.6"
map.has("name")
map.keys()
map.values()
map.remove("name")
map.clear()
```

## Conversion (globals)

```
to_string(42)        // "42"
parse_int("42")      // 42
parse_float("3.14")  // 3.14
len(arr_or_str)      // length (also `.len` property)
```

## Time (globals)

```
time_now()           // epoch ms
time_now_secs()      // epoch seconds
time_year()
time_month()
time_day()
time_hour()
time_minute()
time_second()
time_date()          // "YYYY-MM-DD"
time_datetime()      // "YYYY-MM-DD HH:MM:SS"
time_elapsed()
time_sleep(ms)
```

## IoT HAL (globals, simulated by default)

```
gpio_write(pin, level)
gpio_read(pin)
adc_read(channel)
delay_ms(ms)
```

## Next Steps

- [FFI](/en/guide/ffi) - Language integration
- [IoT](/en/guide/iot) - Embedded deployment
