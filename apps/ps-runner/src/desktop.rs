use windows::{
    core::s,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleA,
        UI::WindowsAndMessaging::{
            CreateWindowExA, DefWindowProcA, DestroyWindow, GetSystemMetrics, PostQuitMessage,
            RegisterClassA, SM_CXSCREEN, SM_CYSCREEN, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND,
            WNDCLASSA, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
            WS_VISIBLE,
        },
    },
};

pub fn create_overlay_window() -> (HWND, i32, i32) {
    unsafe {
        let instance: HINSTANCE = GetModuleHandleA(None).unwrap().into();
        let class_name = s!("PixelShell");

        let wc = WNDCLASSA {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassA(&wc);

        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);

        let hwnd = CreateWindowExA(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            s!(""),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            w,
            h,
            None,
            None,
            instance,
            None,
        );

        (hwnd, w, h)
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        _ => DefWindowProcA(hwnd, msg, w, l),
    }
}
