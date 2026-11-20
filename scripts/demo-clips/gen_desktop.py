"""Compose the demo desktop the website clips are rendered over (no real data).

python -I gen_desktop.py <site-shots demo dir> <out.png>
Uses the site-shots demo renders (code editor, dashboard, wallpaper) plus a small chat
window drawn here with fictional names and a fake token, for the redaction clip.
Also writes <out>.windows.json with the window rects (for window hover highlights).
"""
import json
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

DEMO, OUT = sys.argv[1], sys.argv[2]
W, H = 1920, 1200
F = "C:/Windows/Fonts/"
UI = lambda s: ImageFont.truetype(F + "segoeui.ttf", s)
UISB = lambda s: ImageFont.truetype(F + "seguisb.ttf", s)
MONO = lambda s: ImageFont.truetype(F + "consola.ttf", s)

desk = Image.open(f"{DEMO}/lake-dusk.png").convert("RGB").resize((W, H))
windows = []


def place(img, x, y, w, title):
    img = img.convert("RGB")
    h = round(img.height * w / img.width)
    img = img.resize((w, h), Image.LANCZOS)
    sh = Image.new("L", (w + 80, h + 80), 0)
    ImageDraw.Draw(sh).rounded_rectangle([40, 48, 40 + w, 48 + h], 10, fill=120)
    sh = sh.filter(ImageFilter.GaussianBlur(18))
    desk.paste((0, 0, 0), (x - 40, y - 40), sh)
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, w - 1, h - 1], 8, fill=255)
    desk.paste(img, (x, y), mask)
    windows.append({"title": title, "rect": [x, y, w, h]})


def chat():
    w, h = 640, 320
    im = Image.new("RGB", (w, h), (250, 250, 251))
    d = ImageDraw.Draw(im)
    d.rectangle([0, 0, w, 40], fill=(240, 241, 244))
    d.text((16, 9), "Team Chat - #launch", font=UI(15), fill=(60, 62, 68))
    y = 60
    msgs = [
        ("Maya", (90, 140, 220), "Demo box is up. Login below, rotate it after.", None),
        ("Maya", (90, 140, 220), None, "user   demo.admin@example.com"),
        ("Maya", (90, 140, 220), None, "token  7Q2f9KxW4bLp81ZrDemoT0kN5vHq3YaM2"),
        ("Theo", (200, 120, 60), "Thanks, screenshotting the setup steps now.", None),
    ]
    last = None
    for who, col, text, code in msgs:
        if who != last:
            d.ellipse([16, y, 48, y + 32], fill=col)
            d.text((26, y + 4), who[0], font=UISB(16), fill="white")
            d.text((62, y - 2), who, font=UISB(16), fill=(30, 32, 36))
            y += 24
        last = who
        if text:
            d.text((62, y), text, font=UI(16), fill=(50, 52, 58))
            y += 34
        if code:
            d.rounded_rectangle([62, y, w - 24, y + 34], 6, fill=(236, 238, 242))
            d.text((74, y + 6), code, font=MONO(16), fill=(40, 42, 48))
            y += 44
        if text and who == "Maya" and code is None:
            pass
    d.rounded_rectangle([16, h - 56, w - 16, h - 16], 8, outline=(210, 212, 218), fill="white")
    d.text((30, h - 46), "Message #launch", font=UI(15), fill=(150, 152, 158))
    return im


place(Image.open(f"{DEMO}/code-render.png"), 70, 60, 1040, "render.rs - orbit")
place(Image.open(f"{DEMO}/dashboard.png"), 860, 250, 980, "Build Times - Dashboard")
place(chat(), 150, 730, 640, "Team Chat - #launch")
desk.save(OUT)
with open(OUT + ".windows.json", "w") as f:
    json.dump(list(reversed(windows)), f, indent=1)  # topmost first
print("wrote", OUT)
