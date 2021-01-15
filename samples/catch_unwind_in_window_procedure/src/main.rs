use std::any::Any;
use std::cell::RefCell;
use std::panic::*;
use std::ptr::{null, null_mut};
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::libloaderapi::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

thread_local! {
    static UNWIND: RefCell<Option<Box<dyn Any + Send>>> = RefCell::new(None);
}

fn to_utf16(src: &str) -> Vec<u16> {
    src.encode_utf16().chain(Some(0)).collect::<Vec<_>>()
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    catch_unwind(|| match msg {
        #[cfg(feature = "panic_create")]
        WM_CREATE => panic!("WM_CREATE"),
        WM_RBUTTONUP => panic!("WM_RBUTTONUP"),
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    })
    .unwrap_or_else(|e| {
        UNWIND.with(|unwind| *unwind.borrow_mut() = Some(e));
        if msg == WM_CREATE {
            -1
        } else {
            0
        }
    })
}

fn main() {
    unsafe {
        let class_name = to_utf16("sample_window");
        let window_name = to_utf16("catch unwind in window procedure");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as _,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(null()),
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: GetStockObject(WHITE_BRUSH as _) as _,
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: null_mut(),
        };
        if RegisterClassExW(&wc) == 0 {
            panic!("RegisterClassEx failed");
        }
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            640,
            480,
            null_mut(),
            null_mut(),
            wc.hInstance,
            null_mut(),
        );
        UNWIND.with(|unwind| {
            if let Some(e) = unwind.borrow_mut().take() {
                resume_unwind(e);
            }
        });
        if hwnd == null_mut() {
            panic!("CreateWindowEx failed");
        }
        ShowWindow(hwnd, SW_SHOW);
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, null_mut(), 0, 0);
            if ret == 0 || ret == -1 {
                break;
            }
            DispatchMessageW(&msg);
            UNWIND.with(|unwind| {
                if let Some(e) = unwind.borrow_mut().take() {
                    resume_unwind(e);
                }
            });
        }
    }
}
