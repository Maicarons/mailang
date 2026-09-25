# Built-in Functions

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [stdlib Source](https://github.com/Maicarons/mailang/tree/master/crates/mailang-stdlib)

## Overview

MaìLang provides global built-in functions, plus built-in methods on arrays, maps, and strings. Only APIs that the current runtime actually supports are listed below.

## Input / Output

### println(...)

Print a line to standard output.

```
println("Hello, World!")
println("数字: {42}")
println("多项: {1}, {2}, {3}")
```

### print(...)

Print to standard output (no newline).

```
print("请输入: ")
```

### input(prompt?)

Read a line from standard input.

```
let name = input("请输入姓名: ")
let line = input()
```

## File I/O

### read_file(path)

Read the entire file contents and return them as a string.

### write_file(path, contents)

Write to a file (overwrite), returning null.

```
let content = read_file("config.json")
write_file("output.txt", "Hello, World!")
```

## Math Functions (global)

```
abs(x)           // absolute value
sqrt(x)          // square root
sin(x)           // sine
cos(x)           // cosine
floor(x)         // floor
ceil(x)          // ceiling
round(x)         // round
min(a, b, ...)   // minimum
max(a, b, ...)   // maximum
```

## Strings

Length uses the `s.len` property (or the global `len(s)`).

### Methods

```
s.trim()                 // trim leading/trailing whitespace
s.to_upper()             // to uppercase (alias to_uppercase)
s.to_lower()             // to lowercase (alias to_lowercase)
s.starts_with(prefix)    // starts with prefix?
s.ends_with(suffix)      // ends with suffix?
s.contains(sub)          // contains substring?
s.split(delimiter)       // split string → array
s.replace(old, new)      // replace matches (all)
s.repeat(n)              // repeat string
```

## Arrays

Length uses the `arr.len` property (or the global `len(arr)`).

### Methods

```
arr.push(x)          // append
arr.pop()            // remove and return last element
arr.insert(i, x)     // insert at index
arr.contains(x)      // contains?
arr.join(sep)        // join to string
arr.reverse()        // reverse
arr.clear()          // clear
```

## Maps

### Methods

```
map.has(key)         // has key?
map.keys()           // all keys → array
map.values()         // all values → array
map.remove(key)      // remove key-value pair
map.clear()          // clear
```

Index access: `map[key]` to read, `map[key] = val` to write.

## Type Conversion (global)

```
to_string(x)         // convert to string
parse_int(s)         // parse string to integer
parse_float(s)       // parse string to float
len(x)               // length of string / array
```

## Time (global)

```
time_now()               // current timestamp (milliseconds)
time_now_secs()          // current timestamp (seconds)
time_year()              // year
time_month()             // month
time_day()               // day
time_hour()              // hour
time_minute()            // minute
time_second()            // second
time_date()              // "YYYY-MM-DD"
time_datetime()          // "YYYY-MM-DD HH:MM:SS"
time_elapsed(start)      // milliseconds elapsed since start
time_sleep(ms)           // sleep
```

## IoT HAL (global, simulated by default)

```
gpio_write(pin, level)
gpio_read(pin)
adc_read(channel)
delay_ms(ms)
```

## Next Steps

- [Error Codes](/v0.3/en/reference/errors) — error code reference
- [Type System](/v0.3/en/reference/types) — type details
