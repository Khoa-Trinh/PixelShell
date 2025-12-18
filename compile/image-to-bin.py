import json
import os

import helpers
from PIL import Image

# Configuration
INPUT_IMAGE = "../assets/download.avif"  # Update filename as needed
JSON_OUTPUT = "../assets/boxes.json"
BIN_OUTPUT = "../assets/boxes.bin"
FRAME_COUNT = 1


def main():
    if not os.path.exists(INPUT_IMAGE):
        print(f"Error: {INPUT_IMAGE} not found.")
        return

    print(f"Processing {INPUT_IMAGE}...")
    im = Image.open(INPUT_IMAGE)

    # Process single frame once
    boxes = helpers.process_video_frame(im)
    print(f"Found {len(boxes)} boxes in the image.")

    # Create a list where this single frame is repeated 500 times
    print(f"Duplicating frame {FRAME_COUNT} times...")
    all_data = [boxes for _ in range(FRAME_COUNT)]

    # Save JSON
    print(f"Saving JSON to {JSON_OUTPUT}...")
    with open(JSON_OUTPUT, "w") as f:
        json.dump(all_data, f)

    # Save Bin
    helpers.save_to_bin(all_data, BIN_OUTPUT)


if __name__ == "__main__":
    main()
