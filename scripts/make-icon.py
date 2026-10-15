#!/usr/bin/env python3
# Draws the app icon (assets/icon-*.png, icon.ico, icon.icns) with Pillow:
# a red circle holding a page with a folded corner and "#tt" on it.
#   python3 scripts/make-icon.py .
import sys
from PIL import Image, ImageDraw, ImageFilter, ImageFont

root = sys.argv[1] if len(sys.argv) > 1 else "."
S = 2048  # drawn at 2x, then scaled down
TOP, BOTTOM = (0xF0, 0x48, 0x3D), (0xB3, 0x1B, 0x16)
RED, FOLD = (0xE0, 0x30, 0x2A), (0xF4, 0xC2, 0xBE)

img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
grad = Image.new("RGBA", (S, S))
gd = ImageDraw.Draw(grad)
for y in range(S):
    t = y / (S - 1)
    gd.line([(0, y), (S, y)], fill=tuple(int(TOP[i] + (BOTTOM[i] - TOP[i]) * t) for i in range(3)) + (255,))
mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).ellipse([64, 64, S - 64, S - 64], fill=255)
img.paste(grad, (0, 0), mask)

# The page, with a soft shadow and a folded top-right corner.
x0, y0, x1, y1, fold = 540, 430, 1508, 1618, 300
shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ImageDraw.Draw(shadow).rounded_rectangle([x0 + 20, y0 + 44, x1 + 20, y1 + 44], 60, fill=(70, 0, 0, 110))
img = Image.alpha_composite(img, shadow.filter(ImageFilter.GaussianBlur(36)))
d = ImageDraw.Draw(img)
d.polygon([(x0, y0), (x1 - fold, y0), (x1, y0 + fold), (x1, y1), (x0, y1)], fill=(255, 255, 255, 255))
d.polygon([(x1 - fold, y0), (x1 - fold, y0 + fold), (x1, y0 + fold)], fill=FOLD + (255,))

# "#tt" centred on the page.
font = ImageFont.truetype(f"{root}/assets/fonts/Inter-Bold.ttf", 500)
box = d.textbbox((0, 0), "#tt", font=font, anchor="ls")
w, h = box[2] - box[0], box[3] - box[1]
cx, cy = (x0 + x1) / 2, (y0 + y1) / 2 + 60
d.text((cx - w / 2 - box[0], cy + h / 2 - box[3]), "#tt", font=font, fill=RED + (255,), anchor="ls")

img = img.resize((1024, 1024), Image.LANCZOS)
img.save(f"{root}/assets/icon-1024.png")
for s in [16, 32, 48, 64, 128, 256, 512]:
    img.resize((s, s), Image.LANCZOS).save(f"{root}/assets/icon-{s}.png")
img.save(f"{root}/assets/icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
img.save(f"{root}/assets/icon.icns")
print("icons written to", f"{root}/assets")
