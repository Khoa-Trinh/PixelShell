#![windows_subsystem = "windows"]

mod audio;
mod desktop;
mod payload;
mod renderer;

use kira::{
    clock::ClockSpeed,
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::streaming::{StreamingSoundData, StreamingSoundSettings},
};
use std::{io::Cursor, mem, ptr, slice, thread, time::Duration};
use windows::Win32::{
    System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST},
    UI::{
        HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
        WindowsAndMessaging::{
            DispatchMessageA, PeekMessageA, TranslateMessage, MSG, PM_REMOVE, WM_QUIT,
        },
    },
};

use ps_core::{file_header, PixelRect};

fn main() {
    let assets = payload::load();

    let bin_data: &'static [u8] = Box::leak(assets.video_data.into_boxed_slice());
    let audio_data: &'static [u8] = Box::leak(assets.audio_data.into_boxed_slice());
    let video_width = assets.width;
    let video_height = assets.height;

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = windows::Win32::Media::timeBeginPeriod(1);
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
        let audio_sys = audio::AudioSystem::new();

        let (fps_bytes, frames_bytes) = bin_data.split_at(file_header::DATA_START);
        let video_fps = u16::from_le_bytes(fps_bytes[0..2].try_into().unwrap()) as f64;

        let frames: &[PixelRect] = slice::from_raw_parts(
            frames_bytes.as_ptr() as *const _,
            frames_bytes.len() / mem::size_of::<PixelRect>(),
        );
        let mut frames_iter = frames.iter();

        let (hwnd, w, h) = desktop::create_overlay_window();
        let mut renderer = renderer::GdiRenderer::new(w, h, video_width, video_height);

        let mut audio_manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
        let clock = audio_manager
            .add_clock(ClockSpeed::TicksPerSecond(video_fps))
            .unwrap();

        let sound_data = StreamingSoundData::from_cursor(
            Cursor::new(audio_data),
            StreamingSoundSettings::new().start_time(clock.time()),
        )
        .unwrap();

        let volume_ctl = audio_sys.get_volume_control();
        audio_manager.play(sound_data).unwrap();
        clock.start().unwrap();

        let mut next_tick = clock.time().ticks;

        'main_loop: loop {
            if let Some(ref v) = volume_ctl {
                let _ = v.SetMasterVolumeLevelScalar(0.2, ptr::null());
                let _ = v.SetMute(false, ptr::null());
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
                        if c.is_frame_end() {
                            break;
                        }
                    }
                    next_tick += 1;
                }

                renderer.clear();
                loop {
                    let c = match frames_iter.next() {
                        Some(val) => val,
                        None => break 'main_loop,
                    };
                    if c.is_frame_end() {
                        break;
                    }

                    renderer.draw_sparse_rect(c.x, c.y, c.w, c.h);
                }
                renderer.present(hwnd);
                next_tick += 1;
            } else {
                thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
