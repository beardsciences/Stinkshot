"""Render clean demo captures for the site-shots History. No real personal data."""
import math
import os
import random
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
F = "C:/Windows/Fonts/"


def font(name, size):
    return ImageFont.truetype(F + name, size)


UI = lambda s: font("segoeui.ttf", s)
UIB = lambda s: font("segoeuib.ttf", s)
UISB = lambda s: font("seguisb.ttf", s) if os.path.exists(F + "seguisb.ttf") else UIB(s)
MONO = lambda s: font("CascadiaMono.ttf", s)


def chrome(img, title, dark=True):
    """Minimal window chrome strip so the capture reads as an app window."""
    d = ImageDraw.Draw(img)
    w = img.width
    bg = (32, 33, 37) if dark else (243, 243, 245)
    fg = (200, 202, 208) if dark else (60, 62, 68)
    d.rectangle([0, 0, w, 44], fill=bg)
    d.text((20, 12), title, font=UI(17), fill=fg)
    for i, x in enumerate((w - 130, w - 85, w - 40)):
        if i == 0:
            d.line([x, 22, x + 12, 22], fill=fg, width=1)
        elif i == 1:
            d.rectangle([x, 16, x + 11, 27], outline=fg)
        else:
            d.line([x, 16, x + 11, 27], fill=fg)
            d.line([x + 11, 16, x, 27], fill=fg)
    return d


# 1. Code editor -------------------------------------------------------------------------
def code_editor():
    img = Image.new("RGB", (1600, 1000), (24, 25, 29))
    d = chrome(img, "render.rs - orbit")
    d.rectangle([0, 44, 260, 1000], fill=(30, 31, 36))
    tree = ["ORBIT", "  src", "    main.rs", "    render.rs", "    scene.rs", "    camera.rs",
            "  shaders", "    sky.wgsl", "    orb.wgsl", "  Cargo.toml", "  README.md"]
    for i, t in enumerate(tree):
        col = (230, 232, 236) if "render.rs" in t else (150, 154, 162)
        if "render.rs" in t:
            d.rectangle([0, 72 + i * 30 - 4, 260, 72 + i * 30 + 24], fill=(44, 46, 54))
        d.text((18, 72 + i * 30), t, font=UI(17) if i else UIB(15), fill=col)
    d.rectangle([260, 44, 1600, 84], fill=(28, 29, 34))
    d.rectangle([260, 44, 470, 84], fill=(24, 25, 29))
    d.text((282, 54), "render.rs", font=UI(17), fill=(230, 232, 236))
    d.text((500, 54), "scene.rs", font=UI(17), fill=(130, 134, 142))
    K, T, S, C, N, P, F_ = (198, 146, 255), (102, 204, 255), (152, 221, 130), (110, 116, 128), (255, 184, 108), (220, 222, 228), (255, 213, 120)
    lines = [
        [(C, "/// Draws one frame of the scene into the swap chain.")],
        [(K, "pub fn "), (F_, "draw_frame"), (P, "(ctx: &"), (T, "Gpu"), (P, ", scene: &"), (T, "Scene"), (P, ") -> "), (T, "Result"), (P, "<()> {")],
        [(P, "    "), (K, "let "), (P, "frame = ctx.surface."), (F_, "get_current_texture"), (P, "()?;")],
        [(P, "    "), (K, "let "), (P, "view = frame.texture."), (F_, "create_view"), (P, "(&"), (T, "Default"), (P, "::"), (F_, "default"), (P, "());")],
        [(P, "    "), (K, "let mut "), (P, "enc = ctx.device."), (F_, "create_command_encoder"), (P, "(&"), (T, "Default"), (P, "::"), (F_, "default"), (P, "());")],
        [],
        [(P, "    "), (C, "// Sky first, then the orb with its gel highlight on top.")],
        [(P, "    "), (K, "let "), (P, "clear = "), (T, "Color"), (P, " { r: "), (N, "0.02"), (P, ", g: "), (N, "0.03"), (P, ", b: "), (N, "0.05"), (P, ", a: "), (N, "1.0"), (P, " };")],
        [(P, "    {")],
        [(P, "        "), (K, "let mut "), (P, "pass = enc."), (F_, "begin_render_pass"), (P, "(&"), (F_, "pass_desc"), (P, "(&view, clear));")],
        [(P, "        pass."), (F_, "set_pipeline"), (P, "(&ctx.sky);")],
        [(P, "        pass."), (F_, "draw"), (P, "("), (N, "0"), (P, ".."), (N, "3"), (P, ", "), (N, "0"), (P, ".."), (N, "1"), (P, ");")],
        [(P, "        "), (K, "for "), (P, "orb "), (K, "in "), (P, "&scene.orbs {")],
        [(P, "            pass."), (F_, "set_bind_group"), (P, "("), (N, "0"), (P, ", &orb.bind, &[]);")],
        [(P, "            pass."), (F_, "draw_indexed"), (P, "("), (N, "0"), (P, "..orb.indices, "), (N, "0"), (P, ", "), (N, "0"), (P, ".."), (N, "1"), (P, ");")],
        [(P, "        }")],
        [(P, "    }")],
        [],
        [(P, "    ctx.queue."), (F_, "submit"), (P, "("), (T, "Some"), (P, "(enc."), (F_, "finish"), (P, "()));")],
        [(P, "    frame."), (F_, "present"), (P, "();")],
        [(P, "    "), (T, "Ok"), (P, "(())")],
        [(P, "}")],
        [],
        [(C, "/// Seconds since the scene started, for the slow sky drift.")],
        [(K, "fn "), (F_, "elapsed"), (P, "(start: "), (T, "Instant"), (P, ") -> "), (T, "f32"), (P, " {")],
        [(P, "    start."), (F_, "elapsed"), (P, "()."), (F_, "as_secs_f32"), (P, "()")],
        [(P, "}")],
    ]
    m = MONO(19)
    for i, segs in enumerate(lines):
        y = 104 + i * 32
        d.text((290, y), f"{i + 41:>3}", font=m, fill=(80, 84, 94))
        x = 350
        for col, txt in segs:
            d.text((x, y), txt, font=m, fill=col)
            x += d.textlength(txt, font=m)
    d.rectangle([260, 970, 1600, 1000], fill=(40, 42, 50))
    d.text((280, 975), "main   Ln 44, Col 21   UTF-8   Rust", font=UI(15), fill=(180, 184, 192))
    return img


# 2. Dashboard chart ---------------------------------------------------------------------
def dashboard():
    img = Image.new("RGB", (1600, 1000), (247, 248, 250))
    d = chrome(img, "Build Times - Dashboard", dark=False)
    d.text((60, 80), "Build Times", font=UIB(34), fill=(24, 26, 32))
    d.text((60, 128), "Median clean build, last 12 weeks", font=UI(18), fill=(100, 106, 118))
    cards = [("Median", "4m 12s", "-18%"), ("p95", "6m 40s", "-9%"), ("Cache Hits", "87%", "+6%"), ("Runs", "1,284", "+12%")]
    for i, (k, v, delta) in enumerate(cards):
        x = 60 + i * 375
        d.rounded_rectangle([x, 180, x + 350, 300], 14, fill=(255, 255, 255), outline=(228, 230, 236))
        d.text((x + 24, 198), k, font=UI(17), fill=(100, 106, 118))
        d.text((x + 24, 228), v, font=UIB(36), fill=(24, 26, 32))
        good = delta.startswith("-") if i < 2 else delta.startswith("+")
        d.text((x + 250, 242), delta, font=UIB(18), fill=(46, 160, 90) if good else (210, 80, 70))
    d.rounded_rectangle([60, 330, 1540, 940], 14, fill=(255, 255, 255), outline=(228, 230, 236))
    d.text((90, 352), "Weekly median (minutes)", font=UIB(19), fill=(24, 26, 32))
    x0, y0, x1, y1 = 140, 410, 1500, 870
    for k in range(5):
        y = y1 - k * (y1 - y0) / 4
        d.line([x0, y, x1, y], fill=(236, 238, 242), width=1)
        d.text((96, y - 12), f"{k * 2}", font=UI(16), fill=(130, 136, 148))
    vals = [6.9, 6.6, 6.8, 6.1, 5.9, 5.6, 5.8, 5.0, 4.8, 4.6, 4.3, 4.2]
    bw = (x1 - x0) / len(vals)
    for i, v in enumerate(vals):
        bx = x0 + i * bw + bw * 0.22
        by = y1 - v / 8 * (y1 - y0)
        col = (84, 176, 92) if i == len(vals) - 1 else (170, 214, 160)
        d.rounded_rectangle([bx, by, bx + bw * 0.56, y1], 6, fill=col)
        d.text((bx + 4, y1 + 12), f"W{i + 1}", font=UI(15), fill=(130, 136, 148))
    return img


# 3. UI mockup ---------------------------------------------------------------------------
def ui_mockup():
    img = Image.new("RGB", (1600, 1000), (236, 238, 243))
    d = chrome(img, "Onboarding Flow v3 - Design", dark=False)
    for i in range(3):
        x = 110 + i * 480
        d.rounded_rectangle([x + 6, 136, x + 426, 916], 36, fill=(210, 213, 222))
        d.rounded_rectangle([x, 130, x + 420, 910], 36, fill=(255, 255, 255))
        cx = x + 210
        if i == 0:
            d.ellipse([cx - 70, 250, cx + 70, 390], fill=(127, 211, 106))
            d.ellipse([cx - 40, 270, cx + 0, 310], fill=(200, 240, 190))
            d.text((cx, 450), "Welcome", font=UIB(34), fill=(24, 26, 32), anchor="mm")
            d.text((cx, 500), "Set up takes a minute.", font=UI(19), fill=(110, 116, 128), anchor="mm")
        elif i == 1:
            d.text((x + 40, 200), "Pick a Theme", font=UIB(30), fill=(24, 26, 32))
            for j, (nm, c) in enumerate([("System", (190, 196, 210)), ("Dark", (40, 42, 48)), ("Light", (250, 250, 252))]):
                y = 280 + j * 120
                d.rounded_rectangle([x + 40, y, x + 380, y + 96], 16, outline=(127, 211, 106) if j == 1 else (226, 228, 234), width=3 if j == 1 else 2)
                d.rounded_rectangle([x + 60, y + 20, x + 116, y + 76], 10, fill=c, outline=(210, 212, 220))
                d.text((x + 140, y + 32), nm, font=UI(22), fill=(24, 26, 32))
        else:
            d.text((x + 40, 200), "You're Set", font=UIB(30), fill=(24, 26, 32))
            for j in range(4):
                y = 290 + j * 70
                d.ellipse([x + 40, y, x + 72, y + 32], fill=(127, 211, 106))
                d.line([x + 48, y + 17, x + 54, y + 23, x + 64, y + 10], fill=(255, 255, 255), width=4)
                d.rounded_rectangle([x + 92, y + 8, x + 92 + [250, 200, 230, 170][j], y + 24], 8, fill=(232, 234, 240))
        d.rounded_rectangle([x + 40, 800, x + 380, 860], 30, fill=(127, 211, 106))
        d.text((cx, 830), ["Get Started", "Continue", "Done"][i], font=UIB(22), fill=(20, 40, 20), anchor="mm")
    return img


# 4. Photo-like landscape ----------------------------------------------------------------
def landscape():
    W, H = 1920, 1200
    img = Image.new("RGB", (W, H))
    px = img.load()
    top, mid, low = (40, 52, 110), (236, 132, 98), (252, 206, 150)
    for y in range(H):
        t = y / (H * 0.62)
        if t < 0.6:
            a, b, u = top, mid, t / 0.6
        else:
            a, b, u = mid, low, min(1, (t - 0.6) / 0.4)
        c = tuple(int(a[k] + (b[k] - a[k]) * u) for k in range(3))
        for x in range(W):
            px[x, y] = c
    d = ImageDraw.Draw(img)
    d.ellipse([1240, 560, 1420, 740], fill=(255, 226, 180))
    img = img.filter(ImageFilter.GaussianBlur(2))
    d = ImageDraw.Draw(img)
    random.seed(7)

    def ridge(base, amp, col, seed, rough):
        random.seed(seed)
        pts, y = [], base
        for x in range(0, W + 40, 40):
            y += random.uniform(-rough, rough)
            y = max(base - amp, min(base + amp, y))
            pts.append((x, y + amp * 0.5 * math.sin(x / 260 + seed)))
        d.polygon([(0, H)] + pts + [(W, H)], fill=col)

    ridge(690, 120, (150, 96, 120), 1, 34)
    ridge(760, 100, (104, 70, 102), 2, 30)
    ridge(840, 80, (66, 48, 80), 3, 26)
    d.rectangle([0, 900, W, H], fill=(38, 34, 60))
    for y in range(900, H, 6):
        d.line([0, y, W, y], fill=(48 + (y % 18), 42, 72), width=1)
    d.ellipse([1180, 930, 1480, 950], fill=(220, 160, 130))
    d.ellipse([1240, 975, 1420, 985], fill=(180, 130, 120))
    img = img.filter(ImageFilter.GaussianBlur(1.2))
    return img


# 5. Terminal ----------------------------------------------------------------------------
def terminal():
    img = Image.new("RGB", (1600, 1000), (14, 15, 18))
    d = chrome(img, "cargo build --release")
    m = MONO(20)
    G, Y, W_, D, B = (127, 211, 106), (240, 200, 110), (220, 222, 228), (120, 124, 134), (110, 190, 255)
    lines = [
        [(B, "~/orbit "), (D, "on "), (G, "main "), (W_, "$ cargo build --release")],
        [(G, "   Compiling "), (W_, "proc-macro2 v1.0.89")],
        [(G, "   Compiling "), (W_, "unicode-ident v1.0.13")],
        [(G, "   Compiling "), (W_, "bytemuck v1.19.0")],
        [(G, "   Compiling "), (W_, "glam v0.29.2")],
        [(G, "   Compiling "), (W_, "wgpu-types v23.0.0")],
        [(G, "   Compiling "), (W_, "naga v23.1.0")],
        [(G, "   Compiling "), (W_, "wgpu-core v23.0.1")],
        [(G, "   Compiling "), (W_, "wgpu v23.0.1")],
        [(G, "   Compiling "), (W_, "orbit v0.4.0 (~/orbit)")],
        [(Y, "warning"), (W_, ": unused variable: `dt`")],
        [(D, "  --> src/scene.rs:88:9")],
        [(D, "   |")],
        [(D, "88 |     let dt = clock.tick();")],
        [(D, "   |         ^^ help: prefix it with an underscore: `_dt`")],
        [],
        [(G, "    Finished "), (W_, "`release` profile [optimized] target(s) in 41.07s")],
        [(B, "~/orbit "), (D, "on "), (G, "main "), (W_, "$ ./target/release/orbit --bench")],
        [(W_, "frames      3000")],
        [(W_, "avg frame   2.41 ms")],
        [(W_, "p99 frame   3.96 ms")],
        [(B, "~/orbit "), (D, "on "), (G, "main "), (W_, "$ "), (G, "_")],
    ]
    for i, segs in enumerate(lines):
        x, y = 30, 70 + i * 38
        for col, txt in segs:
            d.text((x, y), txt, font=m, fill=col)
            x += d.textlength(txt, font=m)
    return img


# 6. Release notes article ---------------------------------------------------------------
def article():
    img = Image.new("RGB", (1600, 1000), (255, 255, 255))
    d = chrome(img, "Release Notes - Orbit 0.4", dark=False)
    d.rectangle([0, 44, 1600, 110], fill=(250, 250, 251))
    d.line([0, 110, 1600, 110], fill=(232, 234, 238))
    d.text((300, 64), "Orbit", font=UIB(24), fill=(24, 26, 32))
    for i, t in enumerate(["Docs", "Releases", "Community"]):
        d.text((900 + i * 140, 68), t, font=UI(18), fill=(90, 96, 108))
    d.text((300, 160), "Orbit 0.4", font=UIB(48), fill=(20, 22, 28))
    d.text((300, 228), "Released October 30", font=UI(19), fill=(110, 116, 128))
    body = [
        ("h", "Faster Startup"),
        ("p", "Shader modules are now compiled once and cached on disk. A warm start"),
        ("p", "takes 140 ms, down from 610 ms."),
        ("h", "Smoother Sky"),
        ("p", "The sky gradient is dithered, so dark scenes no longer show bands on"),
        ("p", "8-bit displays."),
        ("h", "Fixes"),
        ("b", "Resizing the window during a frame no longer drops the next one."),
        ("b", "The orb highlight follows the light when the camera orbits."),
        ("b", "Config files with unknown keys load instead of failing."),
    ]
    y = 300
    for kind, t in body:
        if kind == "h":
            y += 20
            d.text((300, y), t, font=UIB(28), fill=(24, 26, 32))
            y += 52
        elif kind == "p":
            d.text((300, y), t, font=UI(21), fill=(60, 64, 74))
            y += 34
        else:
            d.ellipse([304, y + 12, 312, y + 20], fill=(127, 180, 106))
            d.text((330, y), t, font=UI(21), fill=(60, 64, 74))
            y += 38
    return img


# 7. Color palette / notes ---------------------------------------------------------------
def palette():
    img = Image.new("RGB", (1400, 900), (28, 29, 33))
    d = chrome(img, "Brand Colors - Notes")
    d.text((60, 90), "Palette", font=UIB(34), fill=(236, 238, 242))
    sw = [("Gel Green", (127, 211, 106)), ("Deep Moss", (46, 110, 62)), ("Ink", (22, 24, 28)),
          ("Fog", (226, 230, 236)), ("Amber", (255, 184, 92)), ("Sky", (92, 176, 255))]
    for i, (nm, c) in enumerate(sw):
        x, y = 60 + (i % 3) * 430, 170 + (i // 3) * 340
        d.rounded_rectangle([x, y, x + 400, y + 220], 18, fill=c, outline=(60, 62, 70))
        d.text((x, y + 236), nm, font=UIB(22), fill=(236, 238, 242))
        d.text((x, y + 270), "#%02X%02X%02X" % c, font=MONO(19), fill=(150, 154, 162))
    return img


# 8. Calendar week ----------------------------------------------------------------------
def calendar():
    img = Image.new("RGB", (1600, 1000), (255, 255, 255))
    d = chrome(img, "Calendar - Week 45", dark=False)
    d.text((40, 70), "November 2026", font=UIB(30), fill=(24, 26, 32))
    days = ["Mon 2", "Tue 3", "Wed 4", "Thu 5", "Fri 6"]
    x0, y0, cw, rh = 120, 140, 290, 70
    for i, t in enumerate(days):
        d.text((x0 + i * cw + 16, y0), t, font=UISB(19), fill=(80, 86, 98))
    for h in range(9, 21):
        y = y0 + 40 + (h - 9) * rh
        d.line([x0, y, 1580, y], fill=(236, 238, 242))
        if h <= 19:
            d.text((40, y - 10), f"{h}:00", font=UI(16), fill=(140, 146, 158))
    ev = [(0, 9.5, 1, "Standup", (127, 211, 106)), (0, 13, 2, "Shader review", (92, 176, 255)),
          (1, 10, 1.5, "Design crit", (255, 184, 92)), (2, 9.5, 1, "Standup", (127, 211, 106)),
          (2, 14, 3, "Focus: release", (196, 156, 255)), (3, 11, 1, "1:1", (92, 176, 255)),
          (3, 15, 1.5, "Playtest", (255, 184, 92)), (4, 9.5, 1, "Standup", (127, 211, 106)),
          (4, 16, 1, "Ship 0.4", (255, 110, 100))]
    for day, start, dur, name, c in ev:
        x = x0 + day * cw + 6
        y = y0 + 40 + (start - 9) * rh + 3
        light = tuple(int(v + (255 - v) * 0.72) for v in c)
        d.rounded_rectangle([x, y, x + cw - 12, y + dur * rh - 6], 10, fill=light)
        d.rectangle([x, y + 4, x + 5, y + dur * rh - 10], fill=c)
        d.text((x + 18, y + 10), name, font=UISB(18), fill=(30, 32, 38))
    return img


# 9. Kanban board -----------------------------------------------------------------------
def kanban():
    img = Image.new("RGB", (1600, 1000), (22, 23, 27))
    d = chrome(img, "Roadmap - Board")
    cols = [("To Do", ["Dither the sky", "Gamepad support", "Window icon"]),
            ("In Progress", ["Shader cache", "Orbit camera easing"]),
            ("Review", ["Config loader"]),
            ("Done", ["Resize fix", "Highlight follows light", "CI cache"])]
    for i, (name, cards) in enumerate(cols):
        x = 40 + i * 390
        d.rounded_rectangle([x, 80, x + 370, 960], 16, fill=(30, 31, 36))
        d.text((x + 22, 100), name, font=UIB(22), fill=(230, 232, 236))
        d.text((x + 330, 102), str(len(cards)), font=UI(19), fill=(130, 134, 142))
        for j, c in enumerate(cards):
            y = 160 + j * 150
            d.rounded_rectangle([x + 16, y, x + 354, y + 130], 12, fill=(40, 42, 48), outline=(52, 54, 62))
            d.text((x + 36, y + 20), c, font=UISB(20), fill=(232, 234, 238))
            tag = [(127, 211, 106), (92, 176, 255), (255, 184, 92), (196, 156, 255)][(i + j) % 4]
            d.rounded_rectangle([x + 36, y + 70, x + 120, y + 98], 14, fill=tuple(v // 3 for v in tag))
            d.text((x + 78, y + 84), ["core", "ui", "gfx", "ops"][(i + j) % 4], font=UI(15), fill=tag, anchor="mm")
    return img


items = [
    ("code-render.png", code_editor),
    ("dashboard.png", dashboard),
    ("onboarding.png", ui_mockup),
    ("lake-dusk.png", landscape),
    ("terminal.png", terminal),
    ("release-notes.png", article),
    ("palette.png", palette),
    ("calendar.png", calendar),
    ("kanban.png", kanban),
]
for name, fn in items:
    fn().save(os.path.join(OUT, name), optimize=True)
    print("wrote", name)
