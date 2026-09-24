//! Minimal hardware abstraction for IoT hosts.
//!
//! Host firmware implements these traits and registers matching FFI callbacks.
//! The simulated implementation below is used for desktop demos and tests.

use core::cell::RefCell;
use mailang_bytecode::Value;
use std::collections::HashMap;
use std::rc::Rc;

pub type Pin = u8;
pub type Port = u8;
pub type I2cAddr = u8;

pub trait Gpio {
    fn set_mode_output(&mut self, pin: Pin);
    fn set_mode_input(&mut self, pin: Pin);
    fn write(&mut self, pin: Pin, high: bool) -> Result<(), String>;
    fn read(&self, pin: Pin) -> Result<bool, String>;
}

pub trait Delay {
    fn delay_ms(&mut self, ms: u32);
}

pub trait Adc {
    fn read_raw(&mut self, channel: u8) -> Result<u16, String>;
}

pub trait Pwm {
    fn set_duty(&mut self, pin: Pin, duty_16: u16) -> Result<(), String>;
    fn set_freq(&mut self, pin: Pin, hz: u32) -> Result<(), String>;
}

pub trait Uart {
    fn write_bytes(&mut self, port: Port, data: &[u8]) -> Result<(), String>;
    fn read_bytes(&mut self, port: Port, max: usize) -> Result<Vec<u8>, String>;
}

pub trait I2c {
    fn write_read(&mut self, addr: I2cAddr, w: &[u8], rlen: usize) -> Result<Vec<u8>, String>;
}

pub trait Spi {
    fn transfer(&mut self, addr: I2cAddr, tx: &[u8], rx_len: usize) -> Result<Vec<u8>, String>;
}

/// Desktop/simulator HAL: 32 virtual GPIO pins + a tick counter for delay.
/// PWM duty/freq tables, UART loopback buffers, and I2C/SPI echo devices.
#[derive(Debug, Clone)]
pub struct SimulatedHal {
    pins: Rc<RefCell<[bool; 32]>>,
    outputs: Rc<RefCell<[bool; 32]>>,
    ticks_ms: Rc<RefCell<u64>>,
    adc: Rc<RefCell<[u16; 8]>>,
    pwm_duty: Rc<RefCell<[u16; 32]>>,
    pwm_freq: Rc<RefCell<[u32; 32]>>,
    /// Per-port UART RX queue (TX is looped back into RX).
    uart_rx: Rc<RefCell<HashMap<Port, Vec<u8>>>>,
    /// Last I2C write per device address.
    i2c_writes: Rc<RefCell<HashMap<I2cAddr, Vec<u8>>>>,
}

impl Default for SimulatedHal {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedHal {
    pub fn new() -> Self {
        Self {
            pins: Rc::new(RefCell::new([false; 32])),
            outputs: Rc::new(RefCell::new([false; 32])),
            ticks_ms: Rc::new(RefCell::new(0)),
            adc: Rc::new(RefCell::new([0; 8])),
            pwm_duty: Rc::new(RefCell::new([0; 32])),
            pwm_freq: Rc::new(RefCell::new([0; 32])),
            uart_rx: Rc::new(RefCell::new(HashMap::new())),
            i2c_writes: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Simulate an external sensor value on an ADC channel.
    pub fn set_adc(&mut self, channel: u8, raw: u16) {
        if (channel as usize) < 8 {
            self.adc.borrow_mut()[channel as usize] = raw;
        }
    }

    /// Externally drive a pin level (as if a button/sensor pulled it).
    pub fn drive_pin(&mut self, pin: Pin, high: bool) {
        if (pin as usize) < 32 {
            self.pins.borrow_mut()[pin as usize] = high;
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        *self.ticks_ms.borrow()
    }

    pub fn pwm_duty(&self, pin: Pin) -> Result<u16, String> {
        let i = Self::check_pin(pin)?;
        Ok(self.pwm_duty.borrow()[i])
    }

    pub fn pwm_freq(&self, pin: Pin) -> Result<u32, String> {
        let i = Self::check_pin(pin)?;
        Ok(self.pwm_freq.borrow()[i])
    }

    pub fn i2c_last_write(&self, addr: I2cAddr) -> Option<Vec<u8>> {
        self.i2c_writes.borrow().get(&addr).cloned()
    }

    fn check_pin(pin: Pin) -> Result<usize, String> {
        if (pin as usize) < 32 {
            Ok(pin as usize)
        } else {
            Err(format!("pin {} out of range (0..31)", pin))
        }
    }
}

impl Gpio for SimulatedHal {
    fn set_mode_output(&mut self, _pin: Pin) {}
    fn set_mode_input(&mut self, _pin: Pin) {}

    fn write(&mut self, pin: Pin, high: bool) -> Result<(), String> {
        let i = Self::check_pin(pin)?;
        self.outputs.borrow_mut()[i] = high;
        self.pins.borrow_mut()[i] = high;
        Ok(())
    }

    fn read(&self, pin: Pin) -> Result<bool, String> {
        let i = Self::check_pin(pin)?;
        Ok(self.pins.borrow()[i])
    }
}

impl Delay for SimulatedHal {
    fn delay_ms(&mut self, ms: u32) {
        *self.ticks_ms.borrow_mut() += ms as u64;
    }
}

impl Adc for SimulatedHal {
    fn read_raw(&mut self, channel: u8) -> Result<u16, String> {
        if (channel as usize) < 8 {
            Ok(self.adc.borrow()[channel as usize])
        } else {
            Err(format!("adc channel {} out of range (0..7)", channel))
        }
    }
}

impl Pwm for SimulatedHal {
    fn set_duty(&mut self, pin: Pin, duty_16: u16) -> Result<(), String> {
        let i = Self::check_pin(pin)?;
        self.pwm_duty.borrow_mut()[i] = duty_16;
        Ok(())
    }

    fn set_freq(&mut self, pin: Pin, hz: u32) -> Result<(), String> {
        let i = Self::check_pin(pin)?;
        self.pwm_freq.borrow_mut()[i] = hz;
        Ok(())
    }
}

impl Uart for SimulatedHal {
    fn write_bytes(&mut self, port: Port, data: &[u8]) -> Result<(), String> {
        self.uart_rx
            .borrow_mut()
            .entry(port)
            .or_default()
            .extend_from_slice(data);
        Ok(())
    }

    fn read_bytes(&mut self, port: Port, max: usize) -> Result<Vec<u8>, String> {
        let mut map = self.uart_rx.borrow_mut();
        let buf = map.entry(port).or_default();
        let n = max.min(buf.len());
        Ok(buf.drain(..n).collect())
    }
}

impl I2c for SimulatedHal {
    fn write_read(&mut self, addr: I2cAddr, w: &[u8], rlen: usize) -> Result<Vec<u8>, String> {
        self.i2c_writes.borrow_mut().insert(addr, w.to_vec());
        // Deterministic echo device: response[i] = addr.wrapping_add(i).
        Ok((0..rlen).map(|i| addr.wrapping_add(i as u8)).collect())
    }
}

impl Spi for SimulatedHal {
    fn transfer(&mut self, _addr: I2cAddr, tx: &[u8], rx_len: usize) -> Result<Vec<u8>, String> {
        // MOSI→MISO loopback, padded/truncated to `rx_len`.
        let mut rx = tx.to_vec();
        rx.resize(rx_len, 0);
        Ok(rx)
    }
}

thread_local! {
    static HAL: RefCell<SimulatedHal> = RefCell::new(SimulatedHal::new());
}

fn pin_arg(args: &[Value], idx: usize, who: &str) -> Result<Pin, String> {
    match args.get(idx) {
        Some(Value::Int(n)) if *n >= 0 && *n <= 255 => Ok(*n as u8),
        _ => Err(format!("{}: pin must be int 0..255", who)),
    }
}

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    if s.len() % 2 != 0 {
        return Err("hex data must have even length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| format!("invalid hex at byte {}", i / 2))
        })
        .collect()
}

fn to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

/// `gpio_write(pin, high)` — pin: int, high: bool
pub fn builtin_gpio_write(args: &[Value]) -> Result<Value, String> {
    let pin = pin_arg(args, 0, "gpio_write(pin, high)")?;
    let high = match args.get(1) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Int(n)) => *n != 0,
        _ => return Err("gpio_write(pin, high): high must be bool or int".into()),
    };
    HAL.with(|h| {
        let mut h = h.borrow_mut();
        h.set_mode_output(pin);
        h.write(pin, high)
    })?;
    Ok(Value::Null)
}

/// `gpio_read(pin)` → bool
pub fn builtin_gpio_read(args: &[Value]) -> Result<Value, String> {
    let pin = pin_arg(args, 0, "gpio_read(pin)")?;
    let v = HAL.with(|h| h.borrow().read(pin))?;
    Ok(Value::Bool(v))
}

/// `delay_ms(ms)` — simulated; advances the HAL clock only.
pub fn builtin_delay_ms(args: &[Value]) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Int(n)) if *n >= 0 => *n as u32,
        _ => return Err("delay_ms(ms): ms must be non-negative int".into()),
    };
    HAL.with(|h| h.borrow_mut().delay_ms(ms));
    Ok(Value::Null)
}

/// `adc_read(channel)` → int raw
pub fn builtin_adc_read(args: &[Value]) -> Result<Value, String> {
    let ch = match args.first() {
        Some(Value::Int(n)) if *n >= 0 && *n <= 255 => *n as u8,
        _ => return Err("adc_read(channel): channel must be int 0..255".into()),
    };
    let v = HAL.with(|h| h.borrow_mut().read_raw(ch))?;
    Ok(Value::Int(v as i64))
}

/// `pwm_write(pin, duty)` — duty is 16-bit (0..=65535).
pub fn builtin_pwm_write(args: &[Value]) -> Result<Value, String> {
    let pin = pin_arg(args, 0, "pwm_write(pin, duty)")?;
    let duty = match args.get(1) {
        Some(Value::Int(n)) if *n >= 0 && *n <= 65535 => *n as u16,
        _ => return Err("pwm_write(pin, duty): duty must be int 0..65535".into()),
    };
    HAL.with(|h| h.borrow_mut().set_duty(pin, duty))?;
    Ok(Value::Null)
}

/// `pwm_freq(pin, hz)`
pub fn builtin_pwm_freq(args: &[Value]) -> Result<Value, String> {
    let pin = pin_arg(args, 0, "pwm_freq(pin, hz)")?;
    let hz = match args.get(1) {
        Some(Value::Int(n)) if *n >= 0 && *n <= u32::MAX as i64 => *n as u32,
        _ => return Err("pwm_freq(pin, hz): hz must be non-negative int".into()),
    };
    HAL.with(|h| h.borrow_mut().set_freq(pin, hz))?;
    Ok(Value::Null)
}

/// `uart_write(port, s)` — writes UTF-8 bytes of `s` (looped back in sim).
pub fn builtin_uart_write(args: &[Value]) -> Result<Value, String> {
    let port = match args.first() {
        Some(Value::Int(n)) if *n >= 0 && *n <= 255 => *n as u8,
        _ => return Err("uart_write(port, s): port must be int 0..255".into()),
    };
    let s = match args.get(1) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Err("uart_write(port, s): s must be string".into()),
    };
    HAL.with(|h| h.borrow_mut().write_bytes(port, s.as_bytes()))?;
    Ok(Value::Null)
}

/// `uart_read(port, n)` → string of up to `n` bytes (UTF-8 lossy).
pub fn builtin_uart_read(args: &[Value]) -> Result<Value, String> {
    let port = match args.first() {
        Some(Value::Int(n)) if *n >= 0 && *n <= 255 => *n as u8,
        _ => return Err("uart_read(port, n): port must be int 0..255".into()),
    };
    let n = match args.get(1) {
        Some(Value::Int(n)) if *n >= 0 => *n as usize,
        _ => return Err("uart_read(port, n): n must be non-negative int".into()),
    };
    let bytes = HAL.with(|h| h.borrow_mut().read_bytes(port, n))?;
    Ok(Value::Str(
        String::from_utf8_lossy(&bytes).into_owned().into(),
    ))
}

/// `i2c_xfer(addr, hexdata, rlen)` → hex string of `rlen` response bytes.
pub fn builtin_i2c_xfer(args: &[Value]) -> Result<Value, String> {
    let addr = match args.first() {
        Some(Value::Int(n)) if *n >= 0 && *n <= 255 => *n as u8,
        _ => return Err("i2c_xfer(addr, hexdata, rlen): addr must be int 0..255".into()),
    };
    let hex = match args.get(1) {
        Some(Value::Str(s)) => parse_hex(s)?,
        _ => return Err("i2c_xfer(addr, hexdata, rlen): hexdata must be string".into()),
    };
    let rlen = match args.get(2) {
        Some(Value::Int(n)) if *n >= 0 => *n as usize,
        _ => return Err("i2c_xfer(addr, hexdata, rlen): rlen must be non-negative int".into()),
    };
    let rx = HAL.with(|h| h.borrow_mut().write_read(addr, &hex, rlen))?;
    Ok(Value::Str(to_hex(&rx).into()))
}

/// `spi_xfer(addr, hexdata, rxlen)` → hex string of `rxlen` response bytes.
pub fn builtin_spi_xfer(args: &[Value]) -> Result<Value, String> {
    let addr = match args.first() {
        Some(Value::Int(n)) if *n >= 0 && *n <= 255 => *n as u8,
        _ => return Err("spi_xfer(addr, hexdata, rxlen): addr must be int 0..255".into()),
    };
    let hex = match args.get(1) {
        Some(Value::Str(s)) => parse_hex(s)?,
        _ => return Err("spi_xfer(addr, hexdata, rxlen): hexdata must be string".into()),
    };
    let rxlen = match args.get(2) {
        Some(Value::Int(n)) if *n >= 0 => *n as usize,
        _ => return Err("spi_xfer(addr, hexdata, rxlen): rxlen must be non-negative int".into()),
    };
    let rx = HAL.with(|h| h.borrow_mut().transfer(addr, &hex, rxlen))?;
    Ok(Value::Str(to_hex(&rx).into()))
}

/// Test/demo helper: inject a simulated ADC reading.
pub fn sim_set_adc(channel: u8, raw: u16) {
    HAL.with(|h| h.borrow_mut().set_adc(channel, raw));
}

/// Test/demo helper: force an external pin level.
pub fn sim_drive_pin(pin: Pin, high: bool) {
    HAL.with(|h| h.borrow_mut().drive_pin(pin, high));
}

/// Test/demo helper: read simulated elapsed ms.
pub fn sim_elapsed_ms() -> u64 {
    HAL.with(|h| h.borrow().elapsed_ms())
}

/// Test/demo helper: PWM duty currently programmed on `pin`.
pub fn sim_pwm_duty(pin: Pin) -> Result<u16, String> {
    HAL.with(|h| h.borrow().pwm_duty(pin))
}

/// Test/demo helper: PWM frequency currently programmed on `pin`.
pub fn sim_pwm_freq(pin: Pin) -> Result<u32, String> {
    HAL.with(|h| h.borrow().pwm_freq(pin))
}

/// Test/demo helper: last I2C write payload for `addr`.
pub fn sim_i2c_last_write(addr: I2cAddr) -> Option<Vec<u8>> {
    HAL.with(|h| h.borrow().i2c_last_write(addr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpio_roundtrip() {
        builtin_gpio_write(&[Value::Int(3), Value::Bool(true)]).unwrap();
        assert_eq!(
            builtin_gpio_read(&[Value::Int(3)]).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn delay_advances_sim_clock() {
        let before = sim_elapsed_ms();
        builtin_delay_ms(&[Value::Int(50)]).unwrap();
        assert_eq!(sim_elapsed_ms(), before + 50);
    }

    #[test]
    fn adc_sim_inject() {
        sim_set_adc(2, 1234);
        assert_eq!(
            builtin_adc_read(&[Value::Int(2)]).unwrap(),
            Value::Int(1234)
        );
    }

    #[test]
    fn pwm_duty_and_freq() {
        builtin_pwm_write(&[Value::Int(1), Value::Int(30000)]).unwrap();
        builtin_pwm_freq(&[Value::Int(1), Value::Int(1000)]).unwrap();
        assert_eq!(sim_pwm_duty(1).unwrap(), 30000);
        assert_eq!(sim_pwm_freq(1).unwrap(), 1000);
    }

    #[test]
    fn uart_loopback() {
        builtin_uart_write(&[Value::Int(0), Value::Str("hi".into())]).unwrap();
        let got = builtin_uart_read(&[Value::Int(0), Value::Int(16)]).unwrap();
        assert_eq!(got, Value::Str("hi".into()));
    }

    #[test]
    fn i2c_xfer_pattern_and_write_log() {
        let rx = builtin_i2c_xfer(&[
            Value::Int(0x50),
            Value::Str("deadbeef".into()),
            Value::Int(3),
        ])
        .unwrap();
        // Pattern response: addr, addr+1, addr+2 → 50 51 52
        assert_eq!(rx, Value::Str("505152".into()));
        assert_eq!(sim_i2c_last_write(0x50), Some(vec![0xde, 0xad, 0xbe, 0xef]));
    }

    #[test]
    fn spi_xfer_loopback_padded() {
        let rx =
            builtin_spi_xfer(&[Value::Int(0), Value::Str("a1b2".into()), Value::Int(4)]).unwrap();
        assert_eq!(rx, Value::Str("a1b20000".into()));
    }
}
