//! Bundled Win32 backends for the `tui` and `gui` standard-library packs.
//!
//! Both packs ship pure-Mai simulations. A real backend registers functions with
//! exactly the hook names declared in the pack; `run_bytecode` re-applies them
//! over the module's globals, so the hook wins over the stub. The packs keep
//! their own display list / frame buffer, so switching to a real backend does
//! not blind `cmd_*` / `out()`.
//!
//! Windows-only. On other platforms `install` reports that no backend exists.

#![cfg(windows)]

use mailang_core::{bytecode::Value, MailangInterpreter};
use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

fn as_int(v: &Value) -> i64 {
    match v {
        Value::Int(i) => *i,
        Value::Float(f) => *f as i64,
        _ => 0,
    }
}

fn as_text(v: &Value) -> String {
    match v {
        Value::Str(s) => s.to_string(),
        Value::Char(c) => c.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => format!("{:?}", other),
    }
}

fn ok_str(s: impl Into<String>) -> Result<Value, String> {
    Ok(Value::Str(s.into().into()))
}

fn ok_int(i: i64) -> Result<Value, String> {
    Ok(Value::Int(i))
}

fn ok_bool(b: bool) -> Result<Value, String> {
    Ok(Value::Bool(b))
}

fn ok_array(items: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Array(Rc::new(RefCell::new(items))))
}

/// "Nothing available right now" is a *MaìLang* `Err` value, not a host
/// failure: the packs hand these straight to the script, which matches on them.
/// Returning `Err(..)` from the hook instead would abort the program.
fn ok_err(msg: impl Into<String>) -> Result<Value, String> {
    Ok(Value::Err(Box::new(Value::Str(msg.into().into()))))
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Null)
}

/// UTF-8 -> UTF-16 for the wide Win32 entry points.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

// ---------------------------------------------------------------------------
// TUI: real terminal
// ---------------------------------------------------------------------------

mod tui_host {
    use super::*;

    /// Real console geometry, falling back to the 80x24 the packs assume when
    /// stdout is redirected.
    pub fn term_size() -> (i64, i64) {
        use windows_sys::Win32::System::Console::{
            GetConsoleScreenBufferInfo, GetStdHandle, CONSOLE_SCREEN_BUFFER_INFO, STD_OUTPUT_HANDLE,
        };
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if handle.is_null() || handle == usize::MAX as *mut _ {
                return (80, 24);
            }
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
            if GetConsoleScreenBufferInfo(handle, &mut info) == 0 {
                return (80, 24);
            }
            let cols = (info.dwSize.X - info.srWindow.Left) as i64;
            let rows = (info.dwSize.Y - info.srWindow.Top) as i64;
            if cols <= 0 || rows <= 0 {
                (80, 24)
            } else {
                (cols, rows)
            }
        }
    }

    /// One keypress, or `None` when nothing is pending.
    ///
    /// Non-blocking on purpose: a script that polls `read_key()` must not wedge
    /// an automated run, so `GetNumberOfConsoleInputEvents` decides whether a
    /// record is queued before it is read.
    pub fn read_key() -> Option<String> {
        use windows_sys::Win32::System::Console::{
            GetNumberOfConsoleInputEvents, GetStdHandle, ReadConsoleInputW, INPUT_RECORD,
            KEY_EVENT, STD_INPUT_HANDLE,
        };
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            if handle.is_null() || handle == usize::MAX as *mut _ {
                return None;
            }
            let mut count: u32 = 0;
            if GetNumberOfConsoleInputEvents(handle, &mut count) == 0 || count == 0 {
                return None;
            }
            let mut record: INPUT_RECORD = std::mem::zeroed();
            let mut read: u32 = 0;
            if ReadConsoleInputW(handle, &mut record, 1, &mut read) == 0 || read == 0 {
                return None;
            }
            if record.EventType as u32 != KEY_EVENT {
                // Mouse / resize / focus records are not keys.
                return None;
            }
            let key = record.Event.KeyEvent;
            if key.bKeyDown == 0 {
                return None;
            }
            decode_key(&key)
        }
    }

    /// Turns a console key record into the escape sequences the packs expect.
    fn decode_key(key: &windows_sys::Win32::System::Console::KEY_EVENT_RECORD) -> Option<String> {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            VK_BACK, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
        };
        // Virtual-key codes win for control/navigation so a keyboard layout
        // cannot change what a script sees.
        match key.wVirtualKeyCode {
            k if k == VK_UP => return Some("\u{1b}[A".to_string()),
            k if k == VK_DOWN => return Some("\u{1b}[B".to_string()),
            k if k == VK_RIGHT => return Some("\u{1b}[C".to_string()),
            k if k == VK_LEFT => return Some("\u{1b}[D".to_string()),
            k if k == VK_RETURN => return Some("\r".to_string()),
            k if k == VK_ESCAPE => return Some("\u{1b}".to_string()),
            k if k == VK_SPACE => return Some(" ".to_string()),
            k if k == VK_TAB => return Some("\t".to_string()),
            k if k == VK_BACK => return Some("\u{7f}".to_string()),
            _ => {}
        }
        // Reading a union field is unsafe by definition.
        let ch = unsafe { key.uChar.UnicodeChar } as u32;
        match ch {
            0xd => Some("\r".to_string()),
            c if c < 0x20 && c != 0x1b => Some(format!("\u{1b}[{}~", c + 64)),
            0 => None,
            c => char::from_u32(c).map(|c| c.to_string()),
        }
    }

    /// Writes to the process stdout so escapes reach the real console.
    pub fn write(text: &str) -> Result<(), String> {
        use std::io::Write;
        let mut out = std::io::stdout();
        out.write_all(text.as_bytes())
            .map_err(|e| format!("tui_write: {}", e))?;
        out.flush().map_err(|e| format!("tui_write: {}", e))
    }

    pub fn register(interp: &mut MailangInterpreter) {
        interp.register_host_fn("tui_write", |args| {
            write(&as_text(&arg(args, 0)))?;
            ok_str("tty")
        });
        interp.register_host_fn("tui_term_size", |_args| {
            let (cols, rows) = term_size();
            ok_array(vec![Value::Int(cols), Value::Int(rows)])
        });
        interp.register_host_fn("tui_is_tty", |_args| ok_bool(true));
        interp.register_host_fn("tui_read_key", |_args| match read_key() {
            Some(k) if !k.is_empty() => ok_str(k),
            _ => ok_err("tui: no key pending"),
        });
    }
}

// ---------------------------------------------------------------------------
// GUI: real Win32 window + GDI
// ---------------------------------------------------------------------------

mod gui_host {
    use super::*;
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    };

    /// Per-window state: the off-screen DIB the packs render into, plus geometry.
    struct Surface {
        hwnd: HWND,
        dib: HBITMAP,
        width: i64,
        height: i64,
    }

    // HWNDs are handed to the OS and the window procedure is a plain
    // `extern "system"` pointer, so the surface table is process-global.
    thread_local! {
        static SURFACES: RefCell<Vec<Surface>> = const { RefCell::new(Vec::new()) };
    }

    /// Derives the public handle the pack sees from the HWND.
    fn handle_of(hwnd: HWND) -> i64 {
        (hwnd as usize >> 4) as i64 & 0x7FFF_FFFF
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_PAINT => unsafe {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let dc = BeginPaint(hwnd, &mut ps);
                if !dc.is_null() {
                    if let Some((dib, w, h)) = surface_of(hwnd) {
                        BitBlt(dc, 0, 0, w, h, dib, 0, 0, SRCCOPY);
                    }
                    EndPaint(hwnd, &ps);
                }
                0
            },
            // We paint every pixel in the DIB, so skip the erase to avoid flicker.
            WM_ERASEBKGND => 1,
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    fn surface_of(hwnd: HWND) -> Option<(HBITMAP, i32, i32)> {
        SURFACES.with(|s| {
            s.borrow()
                .iter()
                .find(|surf| surf.hwnd == hwnd)
                .map(|surf| (surf.dib, surf.width as i32, surf.height as i32))
        })
    }

    /// Packs an opaque sRGB colour into the `COLORREF` the GDI calls want.
    fn argb(r: i64, g: i64, b: i64, _a: i64) -> u32 {
        0xFF00_0000u32
            | (((b as u32) & 0xFF) << 16)
            | (((g as u32) & 0xFF) << 8)
            | ((r as u32) & 0xFF)
    }

    fn create_window(title: &str, width: i64, height: i64) -> Result<HWND, String> {
        use windows_sys::Win32::{
            Foundation::GetLastError, System::LibraryLoader::GetModuleHandleW,
        };
        let class_name = wide("MailangGuiHost");
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance,
                lpszClassName: class_name.as_ptr(),
                // COLOR_WINDOW + 1
                hbrBackground: (1isize as HBRUSH),
                ..std::mem::zeroed()
            };
            // Idempotent: later windows reuse the already-registered class.
            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                wide(title).as_ptr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width.max(1) as i32,
                height.max(1) as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return Err(format!(
                    "gui_create_window: CreateWindowExW failed ({})",
                    GetLastError()
                ));
            }
            Ok(hwnd)
        }
    }

    /// Creates the 32-bit top-down DIB the packs render into.
    fn create_dib(width: i64, height: i64) -> Result<HBITMAP, String> {
        unsafe {
            let mut bi: BITMAPINFO = std::mem::zeroed();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = width as i32;
            // Negative height = top-down rows, matching the packs' y-down coords.
            bi.bmiHeader.biHeight = -(height as i32);
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB;

            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let dib = CreateDIBSection(
                GetDC(std::ptr::null_mut()),
                &bi,
                DIB_RGB_COLORS,
                std::ptr::addr_of_mut!(bits),
                std::ptr::null_mut(),
                0,
            );
            if dib.is_null() || bits.is_null() {
                return Err("gui: CreateDIBSection failed".into());
            }
            Ok(dib)
        }
    }

    /// Runs `f` with a memory DC over the window's DIB so draws land off-screen.
    fn paint(handle: i64, f: impl FnOnce(HDC)) {
        let found = SURFACES.with(|s| {
            s.borrow()
                .iter()
                .find(|x| handle_of(x.hwnd) == handle)
                .map(|x| (x.dib, x.hwnd))
        });
        if let Some((dib, hwnd)) = found {
            unsafe {
                let screen = GetDC(hwnd);
                if screen.is_null() {
                    return;
                }
                let dc = CreateCompatibleDC(screen);
                if !dc.is_null() {
                    let old = SelectObject(dc, dib);
                    f(dc);
                    GdiFlush();
                    SelectObject(dc, old);
                    DeleteDC(dc);
                }
                ReleaseDC(hwnd, screen);
            }
        }
    }

    /// Converts one queued Win32 message into the pack's
    /// `[kind, x, y, data]` event shape.
    ///
    /// Exactly one message is consumed per call: an earlier version drained the
    /// whole queue but returned only the last event, which silently dropped
    /// input in an interactive loop.
    fn poll_event() -> Result<Value, String> {
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        unsafe {
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                let kind = match msg.message {
                    WM_KEYDOWN => "key",
                    WM_LBUTTONDOWN => "mousedown",
                    WM_LBUTTONUP => "mouseup",
                    WM_MOUSEMOVE => "mousemove",
                    WM_SIZE => "resize",
                    WM_CLOSE => "close",
                    _ => {
                        // Everything else (paint, focus, ...) goes to the wnd proc.
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                        continue;
                    }
                };
                let x = (msg.lParam & 0xFFFF) as i64;
                let y = ((msg.lParam >> 16) & 0xFFFF) as i64;
                let data = match msg.message {
                    // Report the character for WM_KEYDOWN so scripts get text,
                    // not a virtual-key code, which keeps this layout-agnostic.
                    WM_KEYDOWN => {
                        let ch = (msg.wParam & 0xFFFF) as u32;
                        let printable = (0x20..0x7F).contains(&ch) && ch != 0x1B;
                        if printable {
                            char::from_u32(ch)
                                .map(|c| c.to_string())
                                .unwrap_or_default()
                        } else {
                            String::new()
                        }
                    }
                    WM_LBUTTONDOWN | WM_LBUTTONUP => "left".to_string(),
                    WM_SIZE => format!("{}x{}", msg.lParam & 0xFFFF, (msg.lParam >> 16) & 0xFFFF),
                    _ => String::new(),
                };
                return ok_array(vec![
                    Value::Str(kind.into()),
                    Value::Int(x),
                    Value::Int(y),
                    Value::Str(data.into()),
                ]);
            }
        }
        ok_err("gui: no event")
    }

    /// Shows or hides a window. Windows are created hidden so that a build
    /// agent without an interactive desktop can still run the pack's tests.
    fn set_visible(handle: i64, visible: bool) -> Result<Value, String> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOW};
        let found = SURFACES.with(|s| {
            s.borrow()
                .iter()
                .find(|x| handle_of(x.hwnd) == handle)
                .map(|x| x.hwnd)
        });
        match found {
            Some(hwnd) => {
                unsafe {
                    // The window proc's own show/hide bookkeeping is not used, so
                    // pass the intended visibility straight through.
                    let cmd = if visible { SW_SHOW } else { SW_HIDE };
                    ShowWindow(hwnd, cmd);
                }
                ok_str("win32")
            }
            None => ok_err("gui: no such window"),
        }
    }

    pub fn register(interp: &mut MailangInterpreter) {
        interp.register_host_fn("gui_create_window", |args| {
            let title = as_text(&arg(args, 0));
            let w = as_int(&arg(args, 1)).max(1);
            let h = as_int(&arg(args, 2)).max(1);
            let hwnd = create_window(&title, w, h)?;
            let dib = create_dib(w, h)?;
            let handle = handle_of(hwnd);
            SURFACES.with(|s| {
                s.borrow_mut().push(Surface {
                    hwnd,
                    dib,
                    width: w,
                    height: h,
                })
            });
            ok_int(handle)
        });

        interp.register_host_fn("gui_close_window", |args| {
            let handle = as_int(&arg(args, 0));
            SURFACES.with(|s| {
                let mut list = s.borrow_mut();
                match list.iter().position(|x| handle_of(x.hwnd) == handle) {
                    Some(pos) => {
                        let surf = list.remove(pos);
                        unsafe {
                            if !surf.dib.is_null() {
                                DeleteObject(surf.dib);
                            }
                            if !surf.hwnd.is_null() {
                                DestroyWindow(surf.hwnd);
                            }
                        }
                        ok_str("win32")
                    }
                    None => ok_err("gui: no such window"),
                }
            })
        });

        interp.register_host_fn("gui_present", |args| {
            let handle = as_int(&arg(args, 0));
            let found = SURFACES.with(|s| {
                s.borrow()
                    .iter()
                    .find(|x| handle_of(x.hwnd) == handle)
                    .map(|x| x.hwnd)
            });
            match found {
                Some(hwnd) => {
                    unsafe {
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                        UpdateWindow(hwnd);
                        // Drain the queue so the window really paints and the
                        // script stays responsive instead of blocking.
                        let mut msg: MSG = std::mem::zeroed();
                        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                    ok_str("win32")
                }
                None => ok_err("gui: no such window"),
            }
        });

        interp.register_host_fn("gui_draw_rect", |args| {
            let handle = as_int(&arg(args, 0));
            let x = as_int(&arg(args, 1)) as i32;
            let y = as_int(&arg(args, 2)) as i32;
            let w = as_int(&arg(args, 3)) as i32;
            let h = as_int(&arg(args, 4)) as i32;
            let color = argb(
                as_int(&arg(args, 5)),
                as_int(&arg(args, 6)),
                as_int(&arg(args, 7)),
                as_int(&arg(args, 8)),
            );
            paint(handle, |dc| unsafe {
                let brush = CreateSolidBrush(color);
                let rect = RECT {
                    left: x,
                    top: y,
                    right: x + w,
                    bottom: y + h,
                };
                FillRect(dc, &rect, brush);
                DeleteObject(brush);
            });
            ok_str("win32")
        });

        interp.register_host_fn("gui_draw_line", |args| {
            let handle = as_int(&arg(args, 0));
            let x1 = as_int(&arg(args, 1)) as i32;
            let y1 = as_int(&arg(args, 2)) as i32;
            let x2 = as_int(&arg(args, 3)) as i32;
            let y2 = as_int(&arg(args, 4)) as i32;
            let color = argb(
                as_int(&arg(args, 5)),
                as_int(&arg(args, 6)),
                as_int(&arg(args, 7)),
                as_int(&arg(args, 8)),
            );
            paint(handle, |dc| unsafe {
                let pen = CreatePen(PS_SOLID, 1, color);
                let old = SelectObject(dc, pen);
                MoveToEx(dc, x1, y1, std::ptr::null_mut());
                LineTo(dc, x2, y2);
                SelectObject(dc, old);
                DeleteObject(pen);
            });
            ok_str("win32")
        });

        interp.register_host_fn("gui_draw_circle", |args| {
            let handle = as_int(&arg(args, 0));
            let cx = as_int(&arg(args, 1)) as i32;
            let cy = as_int(&arg(args, 2)) as i32;
            let rad = as_int(&arg(args, 3)) as i32;
            let filled = matches!(arg(args, 4), Value::Bool(true));
            let color = argb(
                as_int(&arg(args, 5)),
                as_int(&arg(args, 6)),
                as_int(&arg(args, 7)),
                as_int(&arg(args, 8)),
            );
            paint(handle, |dc| unsafe {
                let brush = CreateSolidBrush(color);
                let pen = CreatePen(PS_SOLID, 1, color);
                let oldb = SelectObject(dc, brush);
                let oldp = SelectObject(dc, pen);
                let x1 = cx - rad;
                let y1 = cy - rad;
                let x2 = cx + rad;
                let y2 = cy + rad;
                if filled {
                    let rect = RECT {
                        left: x1,
                        top: y1,
                        right: x2,
                        bottom: y2,
                    };
                    FillRect(dc, &rect, brush);
                } else {
                    Ellipse(dc, x1, y1, x2, y2);
                }
                SelectObject(dc, oldb);
                SelectObject(dc, oldp);
                DeleteObject(brush);
                DeleteObject(pen);
            });
            ok_str("win32")
        });

        interp.register_host_fn("gui_draw_text", |args| {
            let handle = as_int(&arg(args, 0));
            let x = as_int(&arg(args, 1)) as i32;
            let y = as_int(&arg(args, 2)) as i32;
            let text = as_text(&arg(args, 3));
            let color = argb(
                as_int(&arg(args, 4)),
                as_int(&arg(args, 5)),
                as_int(&arg(args, 6)),
                as_int(&arg(args, 7)),
            );
            paint(handle, |dc| unsafe {
                let font = GetStockObject(DEFAULT_GUI_FONT);
                let old = SelectObject(dc, font);
                SetTextColor(dc, color);
                SetBkMode(dc, TRANSPARENT as i32);
                let utf16 = wide(&text);
                TextOutW(dc, x, y, utf16.as_ptr(), utf16.len() as i32);
                SelectObject(dc, old);
            });
            ok_str("win32")
        });

        interp.register_host_fn("gui_poll_event", |_args| poll_event());
        interp.register_host_fn("gui_show_window", |args| {
            set_visible(as_int(&arg(args, 0)), true)
        });
        interp.register_host_fn("gui_hide_window", |args| {
            set_visible(as_int(&arg(args, 0)), false)
        });
        interp.register_host_fn("gui_load_font", |args| {
            ok_str(format!("gdi:{}", as_text(&arg(args, 0))))
        });
    }
}

// ---------------------------------------------------------------------------
// entry point used by the CLI
// ---------------------------------------------------------------------------

/// Installs the requested backend.
pub fn install(interp: &mut MailangInterpreter, backend: &str) -> Result<(), String> {
    match backend {
        "tui" => {
            tui_host::register(interp);
            Ok(())
        }
        "gui" => {
            gui_host::register(interp);
            Ok(())
        }
        // "sim" (and "none") mean: install nothing, keep the pure-Mai
        // simulation the packs ship with. This is the default.
        "sim" | "none" | "" => Ok(()),
        other => Err(format!("unknown host backend '{}'", other)),
    }
}

/// Backends selectable via `--host`. `sim` is the default and installs nothing;
/// the other two replace the packs' stub functions with real Win32 calls.
pub fn available() -> &'static [&'static str] {
    &["sim", "tui", "gui"]
}
