"""Seed a demo History folder for the History search clip (fictional captures, no real data).

python -I seed_history.py <site-shots demo dir> <history out dir>
Same items as the site-shots profile seed (stinkshot-bench/site-shots/seed.py), dated relative to now.
"""
import datetime as dt
import os
import shutil
import sqlite3
import sys

DEMO, HIST = sys.argv[1], os.path.abspath(sys.argv[2])
shutil.rmtree(HIST, ignore_errors=True)
os.makedirs(os.path.join(HIST, ".stinkshot"))
now = dt.datetime.now().replace(second=0, microsecond=0)
today = lambda h, m: now.replace(hour=h, minute=m)
items = [
    # file, app, title, created, starred, mode, ocr
    ("code-render.png", "Code", "render.rs - orbit", today(9, 14), True, "window",
     "Draws one frame of the scene into the swap chain pub fn draw_frame render pass sky orb"),
    ("dashboard.png", "Browser", "Build Times", today(8, 47), False, "region",
     "Build Times Median clean build last 12 weeks Median 4m 12s p95 6m 40s Cache Hits 87% Runs 1,284"),
    ("terminal.png", "Terminal", "cargo build --release", today(8, 31), False, "window",
     "cargo build --release Compiling wgpu orbit Finished release profile in 41.07s avg frame 2.41 ms"),
    ("onboarding.png", "Design", "Onboarding Flow v3", now - dt.timedelta(days=1, hours=2), True, "region",
     "Welcome Set up takes a minute Get Started Pick a Theme System Dark Light Continue You're Set Done"),
    ("release-notes.png", "Browser", "Release Notes - Orbit 0.4", now - dt.timedelta(days=1, hours=5), False, "scrolling",
     "Orbit 0.4 Released October 30 Faster Startup Shader modules are now compiled once and cached on disk Smoother Sky Fixes"),
    ("lake-dusk.png", "Photos", "Lake at Dusk", now - dt.timedelta(days=3), True, "fullscreen", ""),
    ("palette.png", "Notes", "Brand Colors", now - dt.timedelta(days=4), False, "region",
     "Palette Gel Green #7FD36A Deep Moss Ink Fog Amber Sky"),
    ("calendar.png", "Calendar", "Week 45", now - dt.timedelta(days=5, hours=1), False, "window",
     "November 2026 Standup Shader review Design crit Focus release Playtest Ship 0.4"),
    ("kanban.png", "Browser", "Roadmap", now - dt.timedelta(days=6, hours=3), False, "region",
     "To Do In Progress Review Done Shader cache Orbit camera easing Config loader"),
]
db = sqlite3.connect(os.path.join(HIST, ".stinkshot", "index.sqlite"))
db.executescript("""
CREATE TABLE items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE COLLATE NOCASE,
    name TEXT NOT NULL,
    kind INTEGER NOT NULL,
    created INTEGER NOT NULL,
    width INTEGER NOT NULL DEFAULT 0,
    height INTEGER NOT NULL DEFAULT 0,
    app TEXT NOT NULL DEFAULT '',
    window_title TEXT NOT NULL DEFAULT '',
    starred INTEGER NOT NULL DEFAULT 0,
    ocr_text TEXT,
    bytes INTEGER NOT NULL DEFAULT 0,
    mode TEXT NOT NULL DEFAULT ''
);
CREATE INDEX items_created ON items(created DESC);
CREATE INDEX items_app ON items(app COLLATE NOCASE);
CREATE VIRTUAL TABLE items_fts USING fts5(
    ocr_text, window_title, app, name,
    content='items', content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);
CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
    INSERT INTO items_fts(rowid, ocr_text, window_title, app, name)
    VALUES (new.id, new.ocr_text, new.window_title, new.app, new.name);
END;
CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, ocr_text, window_title, app, name)
    VALUES ('delete', old.id, old.ocr_text, old.window_title, old.app, old.name);
END;
CREATE TRIGGER items_au AFTER UPDATE OF ocr_text, window_title, app, name ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, ocr_text, window_title, app, name)
    VALUES ('delete', old.id, old.ocr_text, old.window_title, old.app, old.name);
    INSERT INTO items_fts(rowid, ocr_text, window_title, app, name)
    VALUES (new.id, new.ocr_text, new.window_title, new.app, new.name);
END;
PRAGMA user_version = 1;
""")
from PIL import Image
stars = []
for fname, app, title, created, star, mode, ocr in reversed(items):
    name = created.strftime("Stinkshot_%Y-%m-%d_%H-%M-%S.png")
    dest = os.path.join(HIST, name)
    shutil.copy(os.path.join(DEMO, fname), dest)
    ts = created.timestamp()
    os.utime(dest, (ts, ts))
    w, h = Image.open(dest).size
    db.execute(
        "INSERT INTO items (path,name,kind,created,width,height,app,window_title,starred,ocr_text,bytes,mode)"
        " VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
        (dest, name, 0, int(ts * 1000), w, h, app, title, int(star), ocr, os.path.getsize(dest), mode),
    )
    if star:
        stars.append(name.lower())
db.commit()
db.close()
import json
with open(os.path.join(HIST, ".stinkshot", "starred.json"), "w") as f:
    json.dump(stars, f)
print("seeded", len(items), "items into", HIST)
