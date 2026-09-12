//! Minimal hardware abstraction for IoT hosts.
//!
//! Host firmware implements these traits and registers matching FFI callbacks.
//! The simulated implementation below is used for desktop demos and tests.

use core::cell::RefCell;
use mailang_bytecode::Value;
use std::rc::Rc;

pub type Pin = u8;

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

/// Desktop/simulator HAL: 32 virtual GPIO pins + a tick counter for delay.
#[derive(Debug, Clone)]
pub struct SimulatedHal {
    pins: Rc<RefCell<[bool; 32]>>,
    outputs: Rc<RefCell<[bool; 32]>>,
    ticks_ms: Rc<RefCell<u64>>,
    adc: Rc<RefCell<[u16; 8]>>,
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

thread_local! {
    static HAL: RefCell<SimulatedHal> = RefCell::new(SimulatedHal::new());
}

/// `gpio_write(pin, high)` — pin: int, high: bool
pub fn builtin_gpio_write(args: &[Value]) -> Result<Value, String> {
    let pin = match args.first() {
        Some(Value::Int(n)) if *n >= 0 && *n <= 255 => *n as u8,
        _ => return Err("gpio_write(pin, high): pin must be int 0..255".into()),
    };
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
    let pin = match args.first() {
        Some(Value::Int(n)) if *n >= 0 && *n <= 255 => *n as u8,
        _ => return Err("gpio_read(pin): pin must be int 0..255".into()),
    };
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
