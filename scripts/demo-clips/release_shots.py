"""Frame the harness's window stills into release screenshots.

    python -I release_shots.py --site-stills <dir> --store-stills <dir> --site-out <dir>
        --store-out <dir> --version X.Y.Z --commit <sha>

scripts/site-media.ps1 runs this after the demo harness (overlay/demo_clips.rs, set `shots`)
has rendered `shots-*.png` twice: at 125 % for the site and at 150 % for the Store.

- site: lossless WebP per window and theme plus shots.json (ids, sizes, bytes), the format of
  the site repo's img-unused/img-src/shots.json. Rounded corners and a hairline outline.
- store: PNG, 2560x1440, the window centred on a plain backdrop (the Store wants 1366x768 or
  more, 1920x1080 or larger preferred, at most 10). `--lang es` only labels README.txt; the
  harness sets the UI language.
"""

import argparse
import json
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

SS = 4  # supersampling for smooth corners

# id, still name, crop box (or None), what it shows
SITE = [
    ("history", "history", None,
     "History window: nine demo captures in a grid, sidebar filters by source app, three starred."),
    ("history-search", "history-search", None,
     "History search for \"shader\": captures found by text inside the images (OCR index)."),
    ("settings-general", "settings-general", None,
     "Settings, General page: startup, theme, language and after-capture options."),
    ("settings-privacy", "settings-privacy", None,
     "Settings, Privacy page: captures stay on this PC, update check off by default."),
    ("settings-recording", "settings-recording", None,
     "Settings, Recording page: frame rate, quality, countdown, overlays, audio."),
    ("about", "settings-about", None,
     "Settings, About page: logo, version, license and source link."),
    ("editor", "editor", None,
     "The capture editor over a demo desktop: a region with an arrow, numbered steps and a label."),
]

# Store order: the first one is the listing's lead picture.
STORE = [
    ("editor", "dark"),
    ("history", "dark"),
    ("history-search", "light"),
    ("settings-privacy", "dark"),
    ("settings-recording", "light"),
    ("editor", "light"),
    ("history", "light"),
    ("about", "dark"),
]
STORE_SIZE = (2560, 1440)
LANG_NAMES = {"en": "English (en)", "es": "Spanish, Latin America (es-419)"}
BACKDROP = {
    "dark": ((16, 26, 20), (34, 58, 42)),
    "light": ((238, 245, 239), (205, 228, 209)),
}


def rounded(img: Image.Image, dark: bool, radius: int) -> Image.Image:
    """Rounded corners and a hairline outline, like a Windows 11 window."""
    img = img.convert("RGBA")
    w, h = img.size
    mask = Image.new("L", (w * SS, h * SS), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, w * SS - 1, h * SS - 1], radius * SS, fill=255)
    mask = mask.resize((w, h), Image.LANCZOS)
    border = Image.new("L", (w * SS, h * SS), 0)
    d = ImageDraw.Draw(border)
    d.rounded_rectangle([0, 0, w * SS - 1, h * SS - 1], radius * SS, fill=255)
    d.rounded_rectangle([SS, SS, w * SS - 1 - SS, h * SS - 1 - SS], (radius - 1) * SS, fill=0)
    border = border.resize((w, h), Image.LANCZOS)
    col = (255, 255, 255, 34) if dark else (0, 0, 0, 40)
    line = Image.new("RGBA", (w, h), col[:3] + (0,))
    line.putalpha(border.point(lambda v: v * col[3] // 255))
    out = Image.alpha_composite(img, line)
    out.putalpha(ImageChops.multiply(out.getchannel("A"), mask))
    return out


def backdrop(theme: str) -> Image.Image:
    w, h = STORE_SIZE
    top, bottom = BACKDROP[theme]
    grad = Image.new("RGB", (1, h))
    for y in range(h):
        t = y / (h - 1)
        grad.putpixel((0, y), tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)))
    return grad.resize((w, h)).convert("RGBA")


def store_picture(win: Image.Image, theme: str) -> Image.Image:
    canvas = backdrop(theme)
    cw, ch = canvas.size
    # Fit inside the canvas with a margin; never upscale.
    k = min(1.0, (cw - 240) / win.width, (ch - 160) / win.height)
    if k < 1.0:
        win = win.resize((round(win.width * k), round(win.height * k)), Image.LANCZOS)
    x, y = (cw - win.width) // 2, (ch - win.height) // 2
    shadow = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    alpha = Image.new("L", canvas.size, 0)
    alpha.paste(win.getchannel("A"), (x, y + 16))
    alpha = alpha.filter(ImageFilter.GaussianBlur(28)).point(lambda v: v * (110 if theme == "dark" else 60) // 255)
    shadow.putalpha(alpha)
    canvas = Image.alpha_composite(canvas, shadow)
    canvas.alpha_composite(win, (x, y))
    return canvas.convert("RGB")


def main() -> None:
    ap = argparse.ArgumentParser()
    # Without --site-stills and --site-out only the Store set is written.
    ap.add_argument("--site-stills", type=Path)
    ap.add_argument("--store-stills", type=Path, required=True)
    ap.add_argument("--site-out", type=Path)
    ap.add_argument("--store-out", type=Path, required=True)
    ap.add_argument("--version", required=True)
    ap.add_argument("--commit", required=True)
    ap.add_argument("--site-scale", type=float, default=1.25)
    ap.add_argument("--store-scale", type=float, default=1.5)
    # UI language the stills were rendered in (STINKSHOT_DEMO_LANG); noted in README.txt.
    ap.add_argument("--lang", default="en")
    args = ap.parse_args()
    if args.site_stills and args.site_out:
        write_site(args)
    write_store(args)


def write_site(args) -> None:
    img_dir = args.site_out / "img"
    img_dir.mkdir(parents=True, exist_ok=True)
    manifest = {
        "note": (f"Stinkshot {args.version} ({args.commit[:8]}) windows, rendered offscreen from the app's own UI "
                 "code by the demo harness (crates/app/src/overlay/demo_clips.rs, set 'shots') with default "
                 "settings. History contents are generated demo images. Rendered at 125%: display at half "
                 "the pixel width for 2x density (e.g. a 1600 px image at 800 CSS px). Written by "
                 "scripts/site-media.ps1; do not edit by hand."),
        "app_version": args.version,
        "app_commit": args.commit,
        "device_scale": args.site_scale,
        "images": [],
    }
    radius = round(8 * args.site_scale)
    for key, still, crop, shows in SITE:
        entry = {"id": key, "shows": shows, "variants": {}}
        for theme in ("dark", "light"):
            src = Image.open(args.site_stills / f"shots-{still}-{theme}.png")
            if crop:
                src = src.crop(crop)
            img = rounded(src, theme == "dark", radius)
            name = f"{key}-{theme}.webp"
            path = img_dir / name
            img.save(path, "WEBP", lossless=True, quality=100, method=6)
            entry["variants"][theme] = {
                "src": f"img/{name}", "width": img.width, "height": img.height,
                "css_width_2x": img.width // 2, "bytes": path.stat().st_size, "quality": "lossless",
            }
            print(f"site   {name:30} {img.width}x{img.height} {path.stat().st_size // 1024} KB")
        manifest["images"].append(entry)
    manifest["not_included"] = {
        "toast": "Toasts are excluded from capture (WDA_EXCLUDEFROMCAPTURE); the harness renders one, but the site does not use it.",
        "trim": "The Trim window only appears after a real screen recording.",
    }
    (args.site_out / "shots.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")


def write_store(args) -> None:
    args.store_out.mkdir(parents=True, exist_ok=True)
    for old in args.store_out.glob("*.png"):
        old.unlink()
    info = {key: (still, shows) for key, still, _, shows in SITE}
    # The Store build has no update check, so its Privacy page reads differently.
    info["settings-privacy"] = ("settings-privacy", "Settings, Privacy page: captures never leave the PC, clipboard history off.")
    sradius = round(8 * args.store_scale)
    lines = [
        f"Microsoft Store screenshots for Stinkshot {args.version} (commit {args.commit[:8]}).",
        "Rendered from the app's own UI as the Store build shows it (no update check, no Update",
        "Now) by scripts/site-media.ps1 (demo harness, set 'shots'): demo content, default settings.",
        "Regenerated on every release; do not edit by hand.",
        f"UI language: {LANG_NAMES.get(args.lang, args.lang)}.",
        f"Each is a PNG, {STORE_SIZE[0]}x{STORE_SIZE[1]}: the window at 150 % on a plain backdrop.",
        "Upload in this order; 01 is the lead picture.",
        "",
    ]
    for i, (key, theme) in enumerate(STORE, 1):
        still, shows = info[key]
        src = Image.open(args.store_stills / f"shots-{still}-{theme}.png")
        pic = store_picture(rounded(src, theme == "dark", sradius), theme)
        path = args.store_out / f"{i:02d}-{key}-{theme}.png"
        pic.save(path, "PNG", optimize=True)
        size = path.stat().st_size
        print(f"store  {path.name:30} {pic.width}x{pic.height} {size // 1024} KB")
        lines.append(f"{path.name}  {pic.width}x{pic.height}, {size // 1024} KB, {theme} theme")
        lines.append(f"    {shows}")
    (args.store_out / "README.txt").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
