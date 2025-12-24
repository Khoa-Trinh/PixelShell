use std::{mem, ptr, slice};
use windows::Win32::{
    Foundation::{COLORREF, HWND, POINT, SIZE},
    Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, SelectObject,
        AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION,
        DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
    },
    UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA},
};

pub struct GdiRenderer {
    mem_dc: HDC,
    hbitmap: HBITMAP,
    old_bitmap: HGDIOBJ,
    buffer: &'static mut [u32],
    screen_w: i32,
    screen_h: i32,
    scale_x: f64,
    scale_y: f64,
}

impl GdiRenderer {
    pub fn new(screen_w: i32, screen_h: i32, base_w: u16, base_h: u16) -> Self {
        unsafe {
            let screen_dc = GetDC(HWND(0));
            let mem_dc = CreateCompatibleDC(screen_dc);

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: screen_w,
                    biHeight: -screen_h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits_ptr: *mut std::ffi::c_void = ptr::null_mut();
            let hbitmap =
                CreateDIBSection(screen_dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0).unwrap();

            let old_bitmap = SelectObject(mem_dc, hbitmap);

            let buffer_size = (screen_w * screen_h) as usize;
            let buffer = slice::from_raw_parts_mut(bits_ptr as *mut u32, buffer_size);

            Self {
                mem_dc,
                hbitmap,
                old_bitmap,
                buffer,
                screen_w,
                screen_h,

                scale_x: screen_w as f64 / base_w as f64,
                scale_y: screen_h as f64 / base_h as f64,
            }
        }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0x00000000);
    }

    pub fn draw_sparse_rect(&mut self, x: u16, y: u16, w: u16, h: u16) {
        let left = (x as f64 * self.scale_x).round() as usize;
        let top = (y as f64 * self.scale_y).round() as usize;
        let w_scaled = (w as f64 * self.scale_x).round() as usize;
        let h_scaled = (h as f64 * self.scale_y).round() as usize;

        let right = (left + w_scaled).min(self.screen_w as usize);
        let bottom = (top + h_scaled).min(self.screen_h as usize);

        if right > left && bottom > top {
            let width = self.screen_w as usize;
            for r in top..bottom {
                let row_offset = r * width;
                self.buffer[row_offset + left..row_offset + right].fill(0xFFFFFFFF);
            }
        }
    }

    pub fn present(&self, hwnd: HWND) {
        let size = SIZE {
            cx: self.screen_w,
            cy: self.screen_h,
        };
        let point = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
            ..Default::default()
        };

        unsafe {
            let _ = UpdateLayeredWindow(
                hwnd,
                None,
                Some(&point),
                Some(&size),
                self.mem_dc,
                Some(&point),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }
}

impl Drop for GdiRenderer {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.mem_dc, self.old_bitmap);
            DeleteObject(self.hbitmap);
            DeleteDC(self.mem_dc);
        }
    }
}
