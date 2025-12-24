// Helper function to draw rectangles on the buffer
pub fn draw_rect(
    buffer: &mut [u32],
    screen_w: usize,
    screen_h: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) {
    let right = (x + w).min(screen_w);
    let bottom = (y + h).min(screen_h);
    // Bounds check to prevent crashes on bad data
    if x >= screen_w || y >= screen_h {
        return;
    }

    for r in y..bottom {
        let row_start = r * screen_w;
        // This is safe because of the min checks above, but buffer must be sized correctly
        if row_start + right <= buffer.len() {
            buffer[row_start + x..row_start + right].fill(0xFFFFFFFF);
        }
    }
}
