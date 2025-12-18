import struct

import numpy as np
from numba import jit, prange
from PIL import Image

# ==================================================================================
# CONFIGURATION
# ==================================================================================
MAX_WIDTH = 1024  # High Resolution
THRESHOLD = int(255 * 0.6)


# ==================================================================================
# 1. CPU IMAGE PROCESSOR (Parallelized)
# ==================================================================================
@jit(nopython=True, parallel=True)
def cpu_process_frame(img_array, width, height, threshold):
    """
    Converts RGB to Binary (Black/White) using CPU Parallelism.
    Returns a 2D array of 0s and 1s.
    """
    # Create output array (0=White, 1=Black)
    output = np.zeros((width, height), dtype=np.uint8)

    # Prange allows Numba to use all CPU cores automatically
    for y in prange(height):
        for x in range(width):
            # Luminosity Grayscale Method
            r = img_array[y, x, 0]
            g = img_array[y, x, 1]
            b = img_array[y, x, 2]

            gray = 0.299 * r + 0.587 * g + 0.114 * b

            # Thresholding
            if gray <= threshold:
                output[x, y] = 1  # Black (Box)
            # Else remains 0 (White/Empty)

    return output


# ==================================================================================
# 2. OPTIMIZED BOX FINDER (Greedy Algorithm)
# ==================================================================================
@jit(nopython=True)
def find_boxes_cpu(pixels, width, height):
    """
    Scans the binary image to find largest rectangles.
    Includes 'Skip Optimization' to jump over found boxes.
    """
    visited = np.zeros((width, height), dtype=np.bool_)
    boxes = []

    for y in range(height):
        x = 0
        while x < width:
            # 1. Skip if empty or already handled
            if visited[x, y] or pixels[x, y] == 0:
                x += 1
                continue

            # 2. Found a black pixel! Expand greedily.
            max_w = 0
            max_h = 0
            max_area = 0

            # Measure max possible width from here
            limit_w = 0
            for temp_x in range(x, width):
                if visited[temp_x, y] or pixels[temp_x, y] == 0:
                    break
                limit_w += 1

            # Scan scanlines downwards
            current_w = limit_w
            for h_scan in range(height - y):
                # Check width of this specific row
                valid_w = 0
                for w_scan in range(current_w):
                    if (
                        visited[x + w_scan, y + h_scan]
                        or pixels[x + w_scan, y + h_scan] == 0
                    ):
                        break
                    valid_w += 1

                # The box is limited by the narrowest row
                if valid_w < current_w:
                    current_w = valid_w

                if current_w == 0:
                    break

                area = current_w * (h_scan + 1)
                if area > max_area:
                    max_area = area
                    max_w = current_w
                    max_h = h_scan + 1

            # 3. Save the best box found
            if max_w > 0 and max_h > 0:
                # Mark as visited
                for vy in range(y, y + max_h):
                    for vx in range(x, x + max_w):
                        visited[vx, vy] = True

                boxes.append((x, y, max_w, max_h))

                # OPTIMIZATION: Jump x forward!
                x += max_w
                continue

            x += 1

    return boxes


# ==================================================================================
# 3. MAIN PIPELINE
# ==================================================================================
def process_video_frame(im: Image.Image) -> list:
    w, h = im.size

    # Resize
    ratio = w / h
    new_h = int(MAX_WIDTH / ratio)
    im = im.resize((MAX_WIDTH, new_h), Image.Resampling.BILINEAR)

    # Convert to Numpy (H, W, 3)
    img_np = np.array(im.convert("RGB"))
    height, width, _ = img_np.shape

    # 1. Parallel CPU Thresholding
    binary_pixels = cpu_process_frame(img_np, width, height, THRESHOLD)

    # 2. Fast Box Finding
    boxes = find_boxes_cpu(binary_pixels, width, height)

    return boxes


def save_to_bin(frames_list, bin_path):
    print(f"Serialising {len(frames_list)} frames to {bin_path}...")

    with open(bin_path, "wb") as f:
        for frame_boxes in frames_list:
            for box in frame_boxes:
                x, y, w, h = box

                # --- BINARY FORMAT FIX ---
                # < = Little Endian
                # H = u16 (0-65535) -> Fixes 1024 resolution issue
                f.write(struct.pack("<HHHH", int(x), int(y), int(w), int(h)))

            # Frame Delimiter (0,0,0,0)
            f.write(struct.pack("<HHHH", 0, 0, 0, 0))

    print("Done.")
