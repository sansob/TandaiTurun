#!/usr/bin/env python3
# Draws the app icon (assets/icon-*.png, icon.ico, icon.icns) with Pillow.
#   python3 scripts/make-icon.py .
import sys
from PIL import Image, ImageDraw, ImageFont
root = sys.argv[1]
S = 2048
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
# A solid circle in the brand red, like Postman's orange badge.
m = 64
d.ellipse([m, m, S - m, S - m], fill=(0xE0, 0x30, 0x2A, 255))
font = ImageFont.truetype(f"{root}/assets/fonts/Inter-Bold.ttf", 860)
text = "#tt"
box = d.textbbox((0, 0), text, font=font, anchor="ls")
w, h = box[2] - box[0], box[3] - box[1]
x = (S - w) / 2 - box[0]
y = (S + h) / 2 - box[3] + 10
d.text((x, y - 0), text, font=font, fill=(255, 255, 255, 255), anchor="ls")
img = img.resize((1024, 1024), Image.LANCZOS)
img.save(f"{root}/assets/icon-1024.png")
for s in [16, 32, 48, 64, 128, 256, 512]:
    img.resize((s, s), Image.LANCZOS).save(f"{root}/assets/icon-{s}.png")
img.save(f"{root}/assets/icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
img.save(f"{root}/assets/icon.icns")
