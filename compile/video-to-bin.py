import json

import cv2
import helpers
from PIL import Image
from tqdm import tqdm

# Configuration
INPUT_VIDEO = "../assets/bad apple.mp4"  # Update filename as needed
JSON_OUTPUT = "../assets/boxes.json"
BIN_OUTPUT = "../assets/boxes.bin"


def main():
    cap = cv2.VideoCapture(INPUT_VIDEO)
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))

    print(f"Processing {INPUT_VIDEO} ({total_frames} frames)...")

    prog = tqdm(total=total_frames)
    all_boxes = []
    image_counter = 0

    try:
        while cap.isOpened():
            ret, cv2_im = cap.read()
            if not ret:
                break

            # Convert BGR (OpenCV) to RGB (PIL)
            converted = cv2.cvtColor(cv2_im, cv2.COLOR_BGR2RGB)
            pil_im = Image.fromarray(converted)

            boxes = helpers.process_video_frame(pil_im)
            all_boxes.append(boxes)

            image_counter += 1
            prog.update()

    finally:
        cap.release()

        # 1. Save Intermediate JSON
        print(f"Saving JSON to {JSON_OUTPUT}...")
        with open(JSON_OUTPUT, "w") as f:
            json.dump(all_boxes, f)

        # 2. Convert JSON to BIN
        helpers.save_to_bin(all_boxes, BIN_OUTPUT)


if __name__ == "__main__":
    main()
