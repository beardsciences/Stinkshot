//! SQLite-backed implementation of `History`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context, bail};
use chrono::{DateTime, Local, TimeZone};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{ItemId, ItemKind, ItemMeta, NewItem, Query, Stats};

const SCHEMA_VERSION: i32 = 1;
const META_DIR: &str = ".stinkshot";
const THUMBS_DIR: &str = "thumbs";
/// Starred file names, kept outside the index so stars survive an index rebuild.
const STARRED_FILE: &str = "starred.json";
const COLS: &str = "id, path, kind, created, width, height, app, window_title, starred, \
                    ocr_text, bytes, mode";

pub struct History {
    folder: PathBuf,
    conn: Mutex<Connection>,
    /// The index was rebuilt after corruption: the next `rescan` takes the files that have
    /// an edit sidecar (only this app writes those) back as this app's captures instead of
    /// marking them `EXTERNAL_MODE`. Everything else stays external: retention must not
    /// delete a picture it cannot prove this app made.
    adopt: std::sync::atomic::AtomicBool,
}

/// `mode` of a file `rescan` found in the folder that this app did not index itself. The
/// folder may be one the user shares with other things (Pictures, Desktop), so retention
/// never deletes these.
pub const EXTERNAL_MODE: &str = "external";

/// The index file itself is damaged, as opposed to locked, unreadable or out of disk space.
fn is_corrupt(e: &anyhow::Error) -> bool {
    e.chain()
        .filter_map(|c| c.downcast_ref::<rusqlite::Error>())
        .any(|e| {
            matches!(
                e.sqlite_error_code(),
                Some(rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase)
            )
        })
}

/// `open_connection`, tried again a few times on a failure that may pass (the file locked
/// by a scanner or a backup, a network drive waking up).
fn open_connection_patiently(db: &Path) -> anyhow::Result<Connection> {
    let mut tries = 0;
    loop {
        match open_connection(db) {
            Ok(c) => return Ok(c),
            Err(e) => {
                tries += 1;
                if tries >= 3 || is_corrupt(&e) || e.to_string().contains("newer than supported") {
                    return Err(e);
                }
                log::warn!("history: index not opened ({e:#}); trying again");
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
        }
    }
}

fn kind_to_i(k: ItemKind) -> i64 {
    match k {
        ItemKind::Image => 0,
        ItemKind::Video => 1,
        ItemKind::Gif => 2,
    }
}

fn kind_from_i(i: i64) -> ItemKind {
    match i {
        1 => ItemKind::Video,
        2 => ItemKind::Gif,
        _ => ItemKind::Image,
    }
}

/// Kind by file extension; None for files the history does not track.
fn kind_for_path(path: &Path) -> Option<ItemKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => Some(ItemKind::Image),
        "mp4" => Some(ItemKind::Video),
        "gif" => Some(ItemKind::Gif),
        _ => None,
    }
}

/// Pixel size from the image header only (no full decode). (0, 0) when unreadable.
fn read_dimensions(path: &Path) -> (u32, u32) {
    let r = image::ImageReader::open(path)
        .and_then(|r| r.with_guessed_format())
        .map_err(anyhow::Error::from)
        .and_then(|r| r.into_dimensions().map_err(anyhow::Error::from));
    match r {
        Ok(d) => d,
        Err(e) => {
            log::debug!("history: no dimensions for {}: {e}", path.display());
            gif_header_dimensions(path).unwrap_or((0, 0))
        }
    }
}

/// Logical screen size straight from a GIF header, for files the decoder rejects.
fn gif_header_dimensions(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut buf = [0u8; 10];
    std::fs::File::open(path).ok()?.read_exact(&mut buf).ok()?;
    if &buf[..4] != b"GIF8" {
        return None;
    }
    Some((
        u32::from(u16::from_le_bytes([buf[6], buf[7]])),
        u32::from(u16::from_le_bytes([buf[8], buf[9]])),
    ))
}

fn file_created_ms(md: &std::fs::Metadata) -> i64 {
    md.created()
        .or_else(|_| md.modified())
        .map(|t| DateTime::<Local>::from(t).timestamp_millis())
        .unwrap_or_else(|_| Local::now().timestamp_millis())
}

fn from_ms(ms: i64) -> DateTime<Local> {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .unwrap_or_else(Local::now)
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn row_to_meta(r: &rusqlite::Row<'_>) -> rusqlite::Result<ItemMeta> {
    Ok(ItemMeta {
        id: r.get(0)?,
        path: PathBuf::from(r.get::<_, String>(1)?),
        kind: kind_from_i(r.get(2)?),
        created: from_ms(r.get(3)?),
        width: r.get::<_, i64>(4)?.max(0) as u32,
        height: r.get::<_, i64>(5)?.max(0) as u32,
        app: r.get(6)?,
        window_title: r.get(7)?,
        starred: r.get::<_, i64>(8)? != 0,
        ocr_text: r.get(9)?,
        bytes: r.get::<_, i64>(10)?.max(0) as u64,
        mode: r.get(11)?,
    })
}

/// Turn user text into a safe FTS5 expression: every word quoted (so operators and
/// punctuation are inert), AND-ed, with a prefix match on the last word. None if the text
/// has no searchable word.
fn fts_expression(text: &str) -> Option<String> {
    let words: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let last = words.len().checked_sub(1)?;
    let parts: Vec<String> = words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            if i == last {
                format!("\"{w}\"*")
            } else {
                format!("\"{w}\"")
            }
        })
        .collect();
    Some(parts.join(" "))
}

fn migrate(conn: &Connection) -> anyhow::Result<()> {
    let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        bail!("history index schema {version} is newer than supported {SCHEMA_VERSION}");
    }
    if version < 1 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(
            "CREATE TABLE items (
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
            END;",
        )?;
        tx.pragma_update(None, "user_version", 1)?;
        tx.commit()?;
    }
    Ok(())
}

fn open_connection(db: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(db).with_context(|| format!("open {}", db.display()))?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    // journal_mode returns a row, so it cannot go through execute().
    let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    log::debug!("history: journal mode {mode}");
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // Deleted rows are overwritten with zeros, so the OCR text and window title of a
    // deleted capture do not stay readable in the index file.
    conn.pragma_update(None, "secure_delete", "ON")?;
    migrate(&conn)?;
    // The same for the search index: without this FTS5 only marks a deleted row's words
    // as gone and keeps them until a later merge. The setting is stored in the index
    // (readable by SQLite 3.42 and later; every build of this app bundles a newer one).
    if let Err(e) = conn.execute(
        "INSERT INTO items_fts(items_fts, rank) VALUES('secure-delete', 1)",
        [],
    ) {
        log::warn!("history: search index secure-delete not set: {e}");
    }
    Ok(conn)
}

fn write_png_atomic(dest: &Path, img: &image::RgbaImage) -> anyhow::Result<()> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = dest.with_extension("png.tmp");
    img.save_with_format(&tmp, image::ImageFormat::Png)
        .with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, dest).with_context(|| format!("replace {}", dest.display()))?;
    Ok(())
}

impl History {
    /// Creates the folder and index if needed; migrates schema. A corrupt index is moved
    /// aside and recreated empty (run `rescan` to refill it). Any other failure to open it
    /// (locked, no access, disk full) is an error and leaves the index alone.
    pub fn open(folder: &Path) -> anyhow::Result<History> {
        let meta = folder.join(META_DIR);
        std::fs::create_dir_all(&meta).with_context(|| format!("create {}", meta.display()))?;
        let db = meta.join("index.sqlite");
        let existed = db.exists();
        let mut fresh = !existed;
        let mut rebuilt = false;
        let conn = match open_connection_patiently(&db) {
            Ok(c) => c,
            Err(e) if !is_corrupt(&e) => return Err(e),
            Err(e) => {
                log::warn!("history: index corrupt ({e:#}); rebuilding");
                rebuilt = true;
                let bad = meta.join("index.sqlite.corrupt");
                let _ = std::fs::remove_file(&bad);
                std::fs::rename(&db, &bad).context("move corrupt index aside")?;
                let _ = std::fs::remove_file(meta.join("index.sqlite-wal"));
                let _ = std::fs::remove_file(meta.join("index.sqlite-shm"));
                fresh = true;
                open_connection(&db)?
            }
        };
        if fresh {
            // Thumbnails are keyed by item id and ids restart in a new index, so every
            // cached thumbnail now belongs to a different (or no) item.
            let _ = std::fs::remove_dir_all(meta.join(THUMBS_DIR));
        }
        let h = History {
            folder: folder.to_path_buf(),
            conn: Mutex::new(conn),
            adopt: std::sync::atomic::AtomicBool::new(rebuilt),
        };
        if !fresh && !meta.join(STARRED_FILE).exists() {
            // Index from an older build: seed the star file so a later rebuild keeps stars.
            h.sync_starred();
        }
        Ok(h)
    }

    /// Writes the starred file names to `.stinkshot/starred.json`, best effort.
    fn sync_starred(&self) {
        let names: Vec<String> = {
            let conn = self.lock();
            let Ok(mut stmt) = conn.prepare("SELECT path FROM items WHERE starred = 1") else {
                return;
            };
            let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) else {
                return;
            };
            rows.flatten()
                .map(|p| file_name_of(Path::new(&p)).to_lowercase())
                .collect()
        };
        let path = self.folder.join(META_DIR).join(STARRED_FILE);
        let result = serde_json::to_vec(&names)
            .map_err(anyhow::Error::from)
            .and_then(|json| {
                let tmp = path.with_extension("json.tmp");
                std::fs::write(&tmp, json)?;
                std::fs::rename(&tmp, &path)?;
                Ok(())
            });
        if let Err(e) = result {
            log::warn!("history: could not write {}: {e:#}", path.display());
        }
    }

    fn saved_stars(&self) -> std::collections::HashSet<String> {
        let path = self.folder.join(META_DIR).join(STARRED_FILE);
        std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok())
            .map(|v| v.into_iter().collect())
            .unwrap_or_default()
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn thumb_path(&self, id: ItemId) -> PathBuf {
        self.folder
            .join(META_DIR)
            .join(THUMBS_DIR)
            .join(format!("{id}.png"))
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// True when `path` is a file directly in this history folder. The index stores
    /// absolute paths, so an index copied or moved from elsewhere (or a damaged one) can
    /// name files in another folder; those rows are never handed out, deleted or opened.
    /// A name with a wildcard is refused too: the shell would expand it when recycling.
    fn owns(&self, path: &Path) -> bool {
        let same = |a: &Path, b: &Path| {
            let key = |p: &Path| -> Vec<String> {
                p.components()
                    .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
                    .collect()
            };
            key(a) == key(b)
        };
        let name = file_name_of(path);
        !name.is_empty()
            && !name.contains(['*', '?'])
            && path.parent().is_some_and(|dir| same(dir, &self.folder))
    }

    /// Moves the write-ahead log into the index file and empties it, best effort, so what
    /// was just deleted is not left readable in the log.
    fn checkpoint(&self) {
        if let Err(e) = self.lock().execute_batch("PRAGMA wal_checkpoint(TRUNCATE)") {
            log::debug!("history: checkpoint: {e}");
        }
    }

    /// Index a file already written into the folder. Fills bytes/created from the file.
    /// Adding a path that is already indexed refreshes that row and returns its id.
    pub fn add(&self, item: NewItem) -> anyhow::Result<ItemId> {
        let md = std::fs::metadata(&item.path)
            .with_context(|| format!("stat {}", item.path.display()))?;
        let kind = item
            .kind
            .or_else(|| kind_for_path(&item.path))
            .unwrap_or(ItemKind::Image);
        let (mut w, mut h) = (item.width, item.height);
        if (w == 0 || h == 0) && kind != ItemKind::Video {
            (w, h) = read_dimensions(&item.path);
        }
        let path = item.path.to_string_lossy().into_owned();
        let name = file_name_of(&item.path);
        let created = file_created_ms(&md);
        let bytes = md.len() as i64;
        let conn = self.lock();
        let existing: Option<(ItemId, bool)> = conn
            .query_row(
                "SELECT id, starred FROM items WHERE path = ?1",
                [&path],
                |r| Ok((r.get(0)?, r.get::<_, i64>(1)? != 0)),
            )
            .optional()?;
        if let Some((id, was_starred)) = existing {
            // A new capture under a name the index still knew: nothing of the old one
            // carries over. The time is the last write, because Windows gives a file
            // recreated under a recently deleted name the old creation time.
            let written = md
                .modified()
                .map(|t| DateTime::<Local>::from(t).timestamp_millis())
                .unwrap_or(created);
            conn.execute(
                "UPDATE items SET kind=?2, width=?3, height=?4, app=?5, window_title=?6, \
                 mode=?7, bytes=?8, name=?9, ocr_text=NULL, created=?10, starred=0 \
                 WHERE id=?1",
                params![
                    id,
                    kind_to_i(kind),
                    w,
                    h,
                    item.app,
                    item.window_title,
                    item.mode,
                    bytes,
                    name,
                    written
                ],
            )?;
            drop(conn);
            let _ = std::fs::remove_file(self.thumb_path(id));
            if was_starred {
                self.sync_starred();
            }
            return Ok(id);
        }
        conn.execute(
            "INSERT INTO items (path, name, kind, created, width, height, app, window_title, \
             bytes, mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                path,
                name,
                kind_to_i(kind),
                created,
                w,
                h,
                item.app,
                item.window_title,
                bytes,
                item.mode
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get(&self, id: ItemId) -> anyhow::Result<Option<ItemMeta>> {
        let conn = self.lock();
        Ok(conn
            .query_row(
                &format!("SELECT {COLS} FROM items WHERE id = ?1"),
                [id],
                row_to_meta,
            )
            .optional()?
            .filter(|m| self.owns(&m.path)))
    }

    pub fn find_by_path(&self, path: &Path) -> anyhow::Result<Option<ItemMeta>> {
        let conn = self.lock();
        Ok(conn
            .query_row(
                &format!("SELECT {COLS} FROM items WHERE path = ?1"),
                [path.to_string_lossy().as_ref()],
                row_to_meta,
            )
            .optional()?
            .filter(|m| self.owns(&m.path)))
    }

    pub fn query(&self, q: &Query) -> anyhow::Result<Vec<ItemMeta>> {
        let mut sql = format!("SELECT {COLS} FROM items WHERE 1=1");
        let mut args: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(expr) = q.text.as_deref().and_then(fts_expression) {
            sql.push_str(" AND id IN (SELECT rowid FROM items_fts WHERE items_fts MATCH ?)");
            args.push(expr.into());
        }
        if let Some(app) = &q.app {
            sql.push_str(" AND app = ? COLLATE NOCASE");
            args.push(app.clone().into());
        }
        if let Some(k) = q.kind {
            sql.push_str(" AND kind = ?");
            args.push(kind_to_i(k).into());
        }
        if q.starred_only {
            sql.push_str(" AND starred = 1");
        }
        if let Some(since) = q.since {
            sql.push_str(" AND created >= ?");
            args.push(since.timestamp_millis().into());
        }
        sql.push_str(" ORDER BY created DESC, id DESC");
        if q.limit > 0 {
            sql.push_str(" LIMIT ? OFFSET ?");
            args.push((q.limit as i64).into());
            args.push((q.offset as i64).into());
        } else if q.offset > 0 {
            sql.push_str(" LIMIT -1 OFFSET ?");
            args.push((q.offset as i64).into());
        }
        let conn = self.lock();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(args), row_to_meta)?;
        let mut items = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        items.retain(|m| self.owns(&m.path));
        Ok(items)
    }

    /// Distinct apps with item counts, most frequent first. Items without an app are
    /// not listed.
    pub fn apps(&self) -> anyhow::Result<Vec<(String, u64)>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT app, COUNT(*) AS n FROM items WHERE app <> '' \
             GROUP BY app COLLATE NOCASE ORDER BY n DESC, app COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// `recordings` counts videos and GIFs.
    pub fn stats(&self) -> anyhow::Result<Stats> {
        let conn = self.lock();
        Ok(conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(starred),0), \
             COALESCE(SUM(kind IN (1,2)),0), COALESCE(SUM(bytes),0) FROM items",
            [],
            |r| {
                Ok(Stats {
                    count: r.get::<_, i64>(0)? as u64,
                    starred: r.get::<_, i64>(1)? as u64,
                    recordings: r.get::<_, i64>(2)? as u64,
                    bytes: r.get::<_, i64>(3)? as u64,
                })
            },
        )?)
    }

    pub fn set_starred(&self, id: ItemId, starred: bool) -> anyhow::Result<()> {
        self.lock().execute(
            "UPDATE items SET starred = ?2 WHERE id = ?1",
            params![id, starred],
        )?;
        self.sync_starred();
        Ok(())
    }

    pub fn set_ocr_text(&self, id: ItemId, text: &str) -> anyhow::Result<()> {
        self.lock().execute(
            "UPDATE items SET ocr_text = ?2 WHERE id = ?1",
            params![id, text],
        )?;
        Ok(())
    }

    /// File was re-saved (re-edit): refresh size/dimensions, clear OCR text so it re-indexes.
    pub fn touch(&self, id: ItemId, width: u32, height: u32) -> anyhow::Result<()> {
        let Some(item) = self.get(id)? else {
            bail!("history item {id} not found");
        };
        let bytes = std::fs::metadata(&item.path)
            .with_context(|| format!("stat {}", item.path.display()))?
            .len() as i64;
        self.lock().execute(
            "UPDATE items SET width=?2, height=?3, bytes=?4, ocr_text=NULL WHERE id=?1",
            params![id, width, height, bytes],
        )?;
        let _ = std::fs::remove_file(self.thumb_path(id));
        Ok(())
    }

    /// The file was re-compressed losslessly (same pixels, fewer bytes): refresh only its
    /// size. OCR text and the thumbnail stay valid.
    pub fn refresh_bytes(&self, id: ItemId) -> anyhow::Result<()> {
        let Some(item) = self.get(id)? else {
            bail!("history item {id} not found");
        };
        let bytes = std::fs::metadata(&item.path)
            .with_context(|| format!("stat {}", item.path.display()))?
            .len() as i64;
        self.lock()
            .execute("UPDATE items SET bytes=?2 WHERE id=?1", params![id, bytes])?;
        Ok(())
    }

    /// Images with ocr_text NULL, newest first. `limit` 0 = no limit.
    pub fn pending_ocr(&self, limit: usize) -> anyhow::Result<Vec<ItemMeta>> {
        let sql = format!(
            "SELECT {COLS} FROM items WHERE kind = 0 AND ocr_text IS NULL \
             ORDER BY created DESC, id DESC LIMIT ?1"
        );
        let lim: i64 = if limit == 0 { -1 } else { limit as i64 };
        let conn = self.lock();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([lim], row_to_meta)?;
        let mut items = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        items.retain(|m| self.owns(&m.path));
        Ok(items)
    }

    /// Removes the re-edit sidecar and thumbnail for a file, best effort.
    fn remove_artifacts(&self, id: ItemId, path: &Path) {
        let _ = std::fs::remove_file(self.thumb_path(id));
        if !self.owns(path) {
            // Its sidecars, if any, are in a folder that is not this history's.
            return;
        }
        // The sidecar layout belongs to core; a failure there must not block deletion.
        let r = std::panic::catch_unwind(|| stinkshot_core::annotate::delete_editable(path));
        match r {
            Ok(Ok(())) => {}
            Ok(Err(e)) => log::warn!("history: delete sidecar for {}: {e:#}", path.display()),
            Err(_) => log::warn!("history: delete sidecar for {} panicked", path.display()),
        }
    }

    /// Deletes the file, its re-edit sidecar, thumbnail and index row. A row whose path is
    /// not a file of this folder (see `owns`) is left alone.
    pub fn delete(&self, id: ItemId) -> anyhow::Result<()> {
        let r = self.delete_row(id);
        self.checkpoint();
        r
    }

    fn delete_row(&self, id: ItemId) -> anyhow::Result<()> {
        let Some(item) = self.get(id)? else {
            return Ok(());
        };
        match std::fs::remove_file(&item.path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(e).with_context(|| format!("delete {}", item.path.display()));
            }
        }
        self.remove_artifacts(id, &item.path);
        self.lock()
            .execute("DELETE FROM items WHERE id = ?1", [id])?;
        if item.starred {
            self.sync_starred();
        }
        Ok(())
    }

    /// Bytes a row really occupies: the file plus its re-edit sidecars and thumbnail.
    fn disk_size(&self, id: ItemId, path: &Path, bytes: i64) -> i64 {
        let side: u64 = stinkshot_core::annotate::editable_files(path)
            .iter()
            .chain(std::iter::once(&self.thumb_path(id)))
            .filter_map(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();
        bytes.saturating_add(i64::try_from(side).unwrap_or(i64::MAX))
    }

    /// Remove non-starred items older than `keep_days` (0 = no age limit), then oldest
    /// non-starred until total bytes <= `max_bytes` (0 = no cap). Returns removed count.
    /// The byte total includes starred items, which are never removed. Files this app did
    /// not create (`EXTERNAL_MODE`) are neither removed nor counted: the cap is on what
    /// Stinkshot keeps, and a total it cannot bring down would cost every own capture.
    pub fn apply_retention(&self, keep_days: u32, max_bytes: u64) -> anyhow::Result<u32> {
        let removed = self.retain(keep_days, max_bytes);
        if !matches!(removed, Ok(0)) {
            self.checkpoint();
        }
        removed
    }

    fn retain(&self, keep_days: u32, max_bytes: u64) -> anyhow::Result<u32> {
        let mut removed = 0u32;
        if keep_days > 0 {
            let cutoff = Local::now().timestamp_millis() - i64::from(keep_days) * 86_400_000;
            let ids: Vec<ItemId> = {
                let conn = self.lock();
                let mut stmt = conn.prepare(
                    "SELECT id, path FROM items \
                     WHERE starred = 0 AND mode <> ?2 AND created < ?1",
                )?;
                let rows = stmt.query_map(params![cutoff, EXTERNAL_MODE], |r| {
                    Ok((r.get::<_, ItemId>(0)?, r.get::<_, String>(1)?))
                })?;
                let rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;
                rows.into_iter()
                    .filter(|(_, p)| self.owns(Path::new(p)))
                    .map(|(id, _)| id)
                    .collect()
            };
            for id in ids {
                match self.delete_row(id) {
                    Ok(()) => removed += 1,
                    Err(e) => log::warn!("history: retention could not delete {id}: {e:#}"),
                }
            }
        }
        if max_bytes > 0 {
            let cap = i64::try_from(max_bytes).unwrap_or(i64::MAX);
            let rows: Vec<(ItemId, PathBuf, i64, bool)> = {
                let conn = self.lock();
                let mut stmt = conn.prepare(
                    "SELECT id, path, bytes, starred FROM items WHERE mode <> ?1 \
                     ORDER BY created ASC, id ASC",
                )?;
                let rows = stmt.query_map([EXTERNAL_MODE], |r| {
                    Ok((
                        r.get(0)?,
                        PathBuf::from(r.get::<_, String>(1)?),
                        r.get(2)?,
                        r.get::<_, i64>(3)? != 0,
                    ))
                })?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            let mut total = 0i64;
            let mut candidates: Vec<(ItemId, i64)> = Vec::new();
            for (id, path, bytes, kept) in rows {
                if !self.owns(&path) {
                    continue;
                }
                let size = self.disk_size(id, &path, bytes);
                total = total.saturating_add(size);
                if !kept {
                    candidates.push((id, size));
                }
            }
            for (id, size) in candidates {
                if total <= cap {
                    break;
                }
                match self.delete_row(id) {
                    Ok(()) => {
                        removed += 1;
                        total -= size;
                    }
                    Err(e) => log::warn!("history: retention could not delete {id}: {e:#}"),
                }
            }
        }
        Ok(removed)
    }

    /// Sync index with folder: add files the user dropped in (png/jpg/webp/mp4/gif, not
    /// under .stinkshot), drop rows whose files vanished. Returns (added, removed).
    /// Added files are marked `EXTERNAL_MODE`, except on the first pass over an index
    /// rebuilt after corruption, where a file with an edit sidecar is this app's capture
    /// coming back. A row that names a file elsewhere, or one that is gone, is pointed at
    /// the file of that name here when there is one (the folder was moved, renamed or
    /// copied), so its app, title, text and star are kept.
    pub fn rescan(&self) -> anyhow::Result<(u32, u32)> {
        // Filesystem work first, without holding the database lock.
        let mut on_disk: Vec<(PathBuf, ItemKind, std::fs::Metadata)> = Vec::new();
        for entry in std::fs::read_dir(&self.folder)
            .with_context(|| format!("read {}", self.folder.display()))?
        {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Ok(md) = entry.metadata() else { continue };
            if !md.is_file() || file_name_of(&path).starts_with('.') {
                continue;
            }
            if let Some(kind) = kind_for_path(&path) {
                on_disk.push((path, kind, md));
            }
        }
        let mut known: HashMap<String, (ItemId, PathBuf)> = {
            let conn = self.lock();
            let mut stmt = conn.prepare("SELECT id, path FROM items")?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, ItemId>(0)?, r.get::<_, String>(1)?)))?;
            let mut m = HashMap::new();
            for r in rows {
                let (id, p) = r?;
                m.insert(p.to_lowercase(), (id, PathBuf::from(p)));
            }
            m
        };

        let moved: Vec<(String, ItemId, PathBuf)> = known
            .iter()
            .filter(|(_, (_, p))| !self.owns(p) || !p.exists())
            .filter_map(|(key, (id, p))| {
                let here = self.folder.join(p.file_name()?);
                (self.owns(&here) && here.is_file()).then(|| (key.clone(), *id, here))
            })
            .collect();
        for (old_key, id, here) in moved {
            let key = here.to_string_lossy().to_lowercase();
            if known.contains_key(&key) {
                continue;
            }
            let res = self.lock().execute(
                "UPDATE items SET path = ?2, name = ?3 WHERE id = ?1",
                params![id, here.to_string_lossy().as_ref(), file_name_of(&here)],
            );
            match res {
                Ok(_) => {
                    known.remove(&old_key);
                    known.insert(key, (id, here));
                }
                Err(e) => log::warn!("history: rescan could not re-point {id}: {e}"),
            }
        }

        let stars = self.saved_stars();
        let adopt = self.adopt.load(std::sync::atomic::Ordering::Relaxed);
        let mut added = 0u32;
        for (path, kind, md) in &on_disk {
            if known.contains_key(&path.to_string_lossy().to_lowercase()) {
                continue;
            }
            let mode = if adopt && stinkshot_core::annotate::has_editable(path) {
                ""
            } else {
                EXTERNAL_MODE
            };
            let (w, h) = if *kind == ItemKind::Video {
                (0, 0)
            } else {
                read_dimensions(path)
            };
            let res = self.lock().execute(
                "INSERT OR IGNORE INTO items \
                 (path, name, kind, created, width, height, bytes, starred, mode) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    path.to_string_lossy().as_ref(),
                    file_name_of(path),
                    kind_to_i(*kind),
                    file_created_ms(md),
                    w,
                    h,
                    md.len() as i64,
                    stars.contains(&file_name_of(path).to_lowercase()),
                    mode
                ],
            );
            match res {
                Ok(n) => added += n as u32,
                Err(e) => log::warn!("history: rescan could not add {}: {e}", path.display()),
            }
        }

        self.adopt
            .store(false, std::sync::atomic::Ordering::Relaxed);

        let mut removed = 0u32;
        for (id, path) in known.values() {
            // A row still naming a file that is not in this folder goes too (the row only).
            if self.owns(path) && path.exists() {
                continue;
            }
            self.remove_artifacts(*id, path);
            match self.lock().execute("DELETE FROM items WHERE id = ?1", [id]) {
                Ok(n) => removed += n as u32,
                Err(e) => log::warn!("history: rescan could not drop {id}: {e}"),
            }
        }
        if removed > 0 {
            self.checkpoint();
        }
        Ok((added, removed))
    }

    /// Cached thumbnail path (generate on first request; videos: None unless cached by
    /// `store_thumbnail`). `max_size` is the longest edge in pixels; an existing cached
    /// thumbnail at least that large (or as large as the source allows) is reused, and
    /// one older than the source file is regenerated.
    pub fn thumbnail(&self, id: ItemId, max_size: u32) -> anyhow::Result<Option<PathBuf>> {
        let Some(item) = self.get(id)? else {
            return Ok(None);
        };
        let thumb = self.thumb_path(id);
        let cached_dims = thumb.exists().then(|| read_dimensions(&thumb));
        if item.kind == ItemKind::Video {
            return Ok(cached_dims.map(|_| thumb));
        }
        let max_size = max_size.clamp(1, 4096);
        let src_md = std::fs::metadata(&item.path)
            .with_context(|| format!("stat {}", item.path.display()))?;
        if let Some((cw, ch)) = cached_dims {
            let fresh = match (
                std::fs::metadata(&thumb).and_then(|m| m.modified()),
                src_md.modified(),
            ) {
                (Ok(t), Ok(s)) => t >= s,
                _ => false,
            };
            let (sw, sh) = read_dimensions(&item.path);
            if fresh && cw.max(ch) >= max_size.min(sw.max(sh)) {
                return Ok(Some(thumb));
            }
        }
        let mut reader = image::ImageReader::open(&item.path)
            .and_then(|r| r.with_guessed_format())
            .with_context(|| format!("open {}", item.path.display()))?;
        // A long scrolling capture is larger than the decoder's default allowance.
        reader.limits(stinkshot_core::image::decode_limits());
        let img = reader
            .decode()
            .with_context(|| format!("decode {}", item.path.display()))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        // Same size contract as a plain resize (fits max_size, never upscales), but it
        // averages blocks first, which is much faster on a big picture.
        let small =
            stinkshot_core::Rgba::from_raw(w, h, img.into_raw()).thumbnail(max_size, max_size);
        self.store_thumbnail(id, &small)?;
        Ok(Some(thumb))
    }

    pub fn store_thumbnail(&self, id: ItemId, thumb: &stinkshot_core::Rgba) -> anyhow::Result<()> {
        let img = image::RgbaImage::from_raw(thumb.width, thumb.height, thumb.data.clone())
            .context("thumbnail buffer does not match its dimensions")?;
        write_png_atomic(&self.thumb_path(id), &img)
    }
}

// ---------------------------------------------------------------------------------------
// Retention preview (read-only), for the confirmation in Settings > History.
// ---------------------------------------------------------------------------------------

impl History {
    /// What `apply_retention(keep_days, max_bytes)` would remove right now, without removing
    /// anything: `(items, bytes freed)`. Same rules and order as `apply_retention`. Stats
    /// files like it does, so call it from a worker thread.
    pub fn retention_preview(&self, keep_days: u32, max_bytes: u64) -> anyhow::Result<(u32, u64)> {
        let cutoff = (keep_days > 0)
            .then(|| Local::now().timestamp_millis() - i64::from(keep_days) * 86_400_000);
        // Like `retain`: files this app did not create, and rows outside this folder, are
        // neither removed nor counted.
        let rows: Vec<(ItemId, PathBuf, i64, i64, bool)> = {
            let conn = self.lock();
            let mut stmt = conn.prepare(
                "SELECT id, path, bytes, created, starred FROM items WHERE mode <> ?1                  ORDER BY created ASC, id ASC",
            )?;
            let rows = stmt.query_map([EXTERNAL_MODE], |r| {
                Ok((
                    r.get(0)?,
                    PathBuf::from(r.get::<_, String>(1)?),
                    r.get(2)?,
                    r.get(3)?,
                    r.get::<_, i64>(4)? != 0,
                ))
            })?;
            let rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;
            rows.into_iter().filter(|r| self.owns(&r.1)).collect()
        };
        let (mut count, mut freed, mut total) = (0u32, 0i64, 0i64);
        let mut candidates: Vec<i64> = Vec::new();
        for (id, path, bytes, created, kept) in rows {
            let aged_out = !kept && cutoff.is_some_and(|c| created < c);
            if !aged_out && max_bytes == 0 {
                continue;
            }
            let size = self.disk_size(id, &path, bytes);
            if aged_out {
                count += 1;
                freed = freed.saturating_add(size);
                continue;
            }
            total = total.saturating_add(size);
            if !kept {
                candidates.push(size);
            }
        }
        if max_bytes > 0 {
            let cap = i64::try_from(max_bytes).unwrap_or(i64::MAX);
            for size in candidates {
                if total <= cap {
                    break;
                }
                count += 1;
                freed = freed.saturating_add(size);
                total -= size;
            }
        }
        Ok((count, u64::try_from(freed).unwrap_or(0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct Tmp(PathBuf);
    impl Tmp {
        fn new() -> Tmp {
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "stinkshot-history-test-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Tmp(p)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
        let p = dir.join(name);
        let img = image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 7, 255])
        });
        img.save(&p).unwrap();
        p
    }

    fn new_item(path: PathBuf, app: &str, title: &str) -> NewItem {
        NewItem {
            path,
            app: app.into(),
            window_title: title.into(),
            mode: "region".into(),
            ..Default::default()
        }
    }

    fn text_query(t: &str) -> Query {
        Query {
            text: Some(t.into()),
            ..Default::default()
        }
    }

    fn set_created(h: &History, id: ItemId, ms: i64) {
        h.lock()
            .execute("UPDATE items SET created=?2 WHERE id=?1", params![id, ms])
            .unwrap();
    }

    #[test]
    fn history_is_send_sync() {
        fn assert_ss<T: Send + Sync>() {}
        assert_ss::<History>();
    }

    #[test]
    fn open_sets_wal_and_version() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let conn = h.lock();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        let v: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, SCHEMA_VERSION);
        drop(conn);
        assert!(t.0.join(".stinkshot").join("index.sqlite").exists());
    }

    #[test]
    fn reopen_keeps_data_and_newer_schema_rejected() {
        let t = Tmp::new();
        let id = {
            let h = History::open(&t.0).unwrap();
            let p = write_png(&t.0, "a.png", 4, 3);
            h.add(new_item(p, "x", "y")).unwrap()
        };
        {
            let h = History::open(&t.0).unwrap();
            assert!(h.get(id).unwrap().is_some());
            h.lock().pragma_update(None, "user_version", 99).unwrap();
        }
        assert!(History::open(&t.0).is_err());
    }

    #[test]
    fn corrupt_index_is_rebuilt() {
        let t = Tmp::new();
        std::fs::create_dir_all(t.0.join(".stinkshot")).unwrap();
        std::fs::write(
            t.0.join(".stinkshot").join("index.sqlite"),
            vec![0x42u8; 4096],
        )
        .unwrap();
        write_png(&t.0, "a.png", 4, 3);
        let h = History::open(&t.0).unwrap();
        assert_eq!(h.rescan().unwrap(), (1, 0));
        assert!(t.0.join(".stinkshot").join("index.sqlite.corrupt").exists());
    }

    #[test]
    fn add_get_find() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let p = write_png(&t.0, "shot.png", 20, 10);
        let id = h
            .add(new_item(p.clone(), "firefox", "Invoice - Mozilla"))
            .unwrap();
        let m = h.get(id).unwrap().unwrap();
        assert_eq!((m.width, m.height), (20, 10));
        assert_eq!(m.kind, ItemKind::Image);
        assert_eq!(m.app, "firefox");
        assert_eq!(m.mode, "region");
        assert_eq!(m.bytes, std::fs::metadata(&p).unwrap().len());
        assert_eq!(m.ocr_text, None);
        assert!(!m.starred);
        assert_eq!(h.find_by_path(&p).unwrap().unwrap().id, id);
        assert!(h.find_by_path(&t.0.join("nope.png")).unwrap().is_none());
        // `path` is COLLATE NOCASE: the default folder's spelling changed once.
        let upper = PathBuf::from(p.to_string_lossy().to_uppercase());
        assert_eq!(h.find_by_path(&upper).unwrap().unwrap().id, id);
        assert!(h.get(9999).unwrap().is_none());
        let id2 = h.add(new_item(p, "chrome", "other")).unwrap();
        assert_eq!(id, id2);
        assert_eq!(h.get(id).unwrap().unwrap().app, "chrome");
        assert_eq!(h.stats().unwrap().count, 1);
        assert!(h.add(new_item(t.0.join("missing.png"), "", "")).is_err());
    }

    #[test]
    fn kind_inference_on_add() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let v = t.0.join("clip.mp4");
        std::fs::write(&v, b"not really video").unwrap();
        let id = h.add(new_item(v, "", "")).unwrap();
        assert_eq!(h.get(id).unwrap().unwrap().kind, ItemKind::Video);
        assert_eq!(h.pending_ocr(10).unwrap().len(), 0);
    }

    #[test]
    fn fts_search_and_prefix() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let a = h
            .add(new_item(
                write_png(&t.0, "a.png", 4, 4),
                "excel",
                "Quarterly report",
            ))
            .unwrap();
        let b = h
            .add(new_item(write_png(&t.0, "b.png", 4, 4), "firefox", "Mail"))
            .unwrap();
        h.set_ocr_text(b, "Please find the invoice number 12345 attached")
            .unwrap();
        let ids = |q: &str| -> Vec<ItemId> {
            h.query(&text_query(q))
                .unwrap()
                .iter()
                .map(|m| m.id)
                .collect()
        };
        assert_eq!(ids("invo"), vec![b]);
        assert_eq!(ids("invoice 123"), vec![b]);
        assert_eq!(ids("invoice zzz"), Vec::<ItemId>::new());
        assert_eq!(ids("quart"), vec![a]);
        assert_eq!(ids("EXCEL"), vec![a]);
        assert_eq!(ids("b.png"), vec![b]);
        h.set_ocr_text(b, "completely different").unwrap();
        assert!(ids("invoice").is_empty());
        assert_eq!(ids("differ"), vec![b]);
        h.delete(b).unwrap();
        assert!(ids("differ").is_empty());
    }

    #[test]
    fn hostile_search_text_is_safe() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let id = h
            .add(new_item(
                write_png(&t.0, "a.png", 4, 4),
                "app",
                "say \"hello\" world",
            ))
            .unwrap();
        for q in [
            "\"",
            "\"\"",
            "AND",
            "OR",
            "NOT",
            "NEAR(",
            "*",
            "a*",
            "(",
            ")",
            "-",
            "x:y",
            "title:hello",
            "' OR 1=1 --",
            "hello\" OR \"",
            "^hello",
            "\u{0}",
            "   ",
            "!!!",
        ] {
            let r = h.query(&text_query(q));
            assert!(r.is_ok(), "query {q:?} failed: {:?}", r.err());
        }
        assert_eq!(h.query(&text_query("\"hello\" wor")).unwrap()[0].id, id);
        assert_eq!(h.query(&text_query("!!!")).unwrap().len(), 1);
        assert!(h.query(&text_query("app:nothing")).unwrap().is_empty());
    }

    #[test]
    fn filters_order_and_paging() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let mut ids = Vec::new();
        for i in 0..5 {
            let app = if i % 2 == 0 { "firefox" } else { "code" };
            let id = h
                .add(new_item(
                    write_png(&t.0, &format!("{i}.png"), 4, 4),
                    app,
                    "",
                ))
                .unwrap();
            set_created(&h, id, 1_000_000 + i64::from(i) * 1000);
            ids.push(id);
        }
        let g = t.0.join("anim.gif");
        std::fs::write(&g, b"GIF89a\x04\x00\x03\x00").unwrap();
        let gid = h.add(new_item(g, "firefox", "")).unwrap();
        set_created(&h, gid, 500);
        h.set_starred(ids[1], true).unwrap();

        let all = h.query(&Query::default()).unwrap();
        assert_eq!(all.len(), 6);
        assert_eq!(all[0].id, ids[4]);
        assert_eq!(all[5].id, gid);

        let page = h
            .query(&Query {
                limit: 2,
                offset: 1,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            page.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![ids[3], ids[2]]
        );
        let tail = h
            .query(&Query {
                offset: 4,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(tail.len(), 2);

        let ff = h
            .query(&Query {
                app: Some("FireFox".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(ff.len(), 4);
        let gifs = h
            .query(&Query {
                kind: Some(ItemKind::Gif),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(gifs.len(), 1);
        let st = h
            .query(&Query {
                starred_only: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(st.len(), 1);
        assert_eq!(st[0].id, ids[1]);
        let since = h
            .query(&Query {
                since: Some(from_ms(1_002_000)),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(since.len(), 3);
        let combo = h
            .query(&Query {
                app: Some("firefox".into()),
                since: Some(from_ms(1_002_000)),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(combo.len(), 2);
        h.set_ocr_text(ids[0], "needle").unwrap();
        let both = h
            .query(&Query {
                text: Some("needle".into()),
                app: Some("code".into()),
                ..Default::default()
            })
            .unwrap();
        assert!(both.is_empty());
    }

    #[test]
    fn apps_and_stats() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        for (i, app) in ["a", "b", "b", "", "c", "b", "a"].iter().enumerate() {
            h.add(new_item(
                write_png(&t.0, &format!("{i}.png"), 4, 4),
                app,
                "",
            ))
            .unwrap();
        }
        let v = t.0.join("v.mp4");
        std::fs::write(&v, b"12345").unwrap();
        let vid = h.add(new_item(v, "b", "")).unwrap();
        h.set_starred(vid, true).unwrap();
        assert_eq!(
            h.apps().unwrap(),
            vec![("b".into(), 4), ("a".into(), 2), ("c".into(), 1)]
        );
        let s = h.stats().unwrap();
        assert_eq!((s.count, s.starred, s.recordings), (8, 1, 1));
        assert!(s.bytes > 5);
        let empty = Tmp::new();
        assert_eq!(
            Stats::default(),
            History::open(&empty.0).unwrap().stats().unwrap()
        );
    }

    #[test]
    fn star_touch_pending_ocr() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let p = write_png(&t.0, "a.png", 4, 4);
        let id = h.add(new_item(p.clone(), "", "")).unwrap();
        h.set_starred(id, true).unwrap();
        assert!(h.get(id).unwrap().unwrap().starred);
        h.set_starred(id, false).unwrap();
        assert!(!h.get(id).unwrap().unwrap().starred);

        assert_eq!(h.pending_ocr(10).unwrap().len(), 1);
        h.set_ocr_text(id, "").unwrap();
        assert_eq!(h.get(id).unwrap().unwrap().ocr_text, Some(String::new()));
        assert!(h.pending_ocr(10).unwrap().is_empty());

        write_png(&t.0, "a.png", 30, 20);
        h.touch(id, 30, 20).unwrap();
        let m = h.get(id).unwrap().unwrap();
        assert_eq!((m.width, m.height), (30, 20));
        assert_eq!(m.ocr_text, None);
        assert_eq!(m.bytes, std::fs::metadata(&p).unwrap().len());
        assert_eq!(h.pending_ocr(10).unwrap().len(), 1);
        assert!(h.touch(777, 1, 1).is_err());
    }

    #[test]
    fn pending_ocr_order_and_limit() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let mut ids = vec![];
        for i in 0..4 {
            let id = h
                .add(new_item(write_png(&t.0, &format!("{i}.png"), 2, 2), "", ""))
                .unwrap();
            set_created(&h, id, 1000 * (i64::from(i) + 1));
            ids.push(id);
        }
        let p = h.pending_ocr(2).unwrap();
        assert_eq!(
            p.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![ids[3], ids[2]]
        );
        assert_eq!(h.pending_ocr(0).unwrap().len(), 4);
    }

    #[test]
    fn delete_removes_everything() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let p = write_png(&t.0, "a.png", 100, 50);
        let id = h.add(new_item(p.clone(), "", "")).unwrap();
        let thumb = h.thumbnail(id, 32).unwrap().unwrap();
        assert!(thumb.exists());
        h.delete(id).unwrap();
        assert!(!p.exists());
        assert!(!thumb.exists());
        assert!(h.get(id).unwrap().is_none());
        h.delete(id).unwrap();
        let p2 = write_png(&t.0, "b.png", 4, 4);
        let id2 = h.add(new_item(p2.clone(), "", "")).unwrap();
        std::fs::remove_file(&p2).unwrap();
        h.delete(id2).unwrap();
        assert!(h.get(id2).unwrap().is_none());
    }

    fn add_sized(
        h: &History,
        dir: &Path,
        name: &str,
        size: usize,
        created: i64,
        starred: bool,
    ) -> ItemId {
        let p = dir.join(name);
        std::fs::write(&p, vec![0u8; size]).unwrap();
        let id = h
            .add(NewItem {
                path: p,
                kind: Some(ItemKind::Image),
                width: 1,
                height: 1,
                ..Default::default()
            })
            .unwrap();
        set_created(h, id, created);
        h.set_starred(id, starred).unwrap();
        id
    }

    #[test]
    fn retention_by_age_spares_starred() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let now = Local::now().timestamp_millis();
        let day = 86_400_000;
        let old = add_sized(&h, &t.0, "old.png", 10, now - 40 * day, false);
        let old_star = add_sized(&h, &t.0, "olds.png", 10, now - 400 * day, true);
        let fresh = add_sized(&h, &t.0, "fresh.png", 10, now - 2 * day, false);
        let edge = add_sized(&h, &t.0, "edge.png", 10, now - 29 * day, false);
        assert_eq!(h.apply_retention(0, 0).unwrap(), 0);
        assert_eq!(h.apply_retention(30, 0).unwrap(), 1);
        assert!(h.get(old).unwrap().is_none());
        assert!(!t.0.join("old.png").exists());
        assert!(h.get(old_star).unwrap().is_some());
        assert!(h.get(fresh).unwrap().is_some());
        assert!(h.get(edge).unwrap().is_some());
    }

    #[test]
    fn retention_by_size_oldest_first() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let a = add_sized(&h, &t.0, "a.png", 100, 1000, false);
        let b = add_sized(&h, &t.0, "b.png", 100, 2000, true);
        let c = add_sized(&h, &t.0, "c.png", 100, 3000, false);
        let d = add_sized(&h, &t.0, "d.png", 100, 4000, false);
        // 400 total, cap 250: remove a, then c (oldest unstarred) -> 200.
        assert_eq!(h.apply_retention(0, 250).unwrap(), 2);
        assert!(h.get(a).unwrap().is_none());
        assert!(h.get(c).unwrap().is_none());
        assert!(h.get(b).unwrap().is_some());
        assert!(h.get(d).unwrap().is_some());
        // Cap below what starred items alone use: remove all unstarred, keep starred.
        assert_eq!(h.apply_retention(0, 50).unwrap(), 1);
        assert!(h.get(d).unwrap().is_none());
        assert!(h.get(b).unwrap().is_some());
        assert_eq!(h.apply_retention(0, 50).unwrap(), 0);
    }

    #[test]
    fn retention_age_then_size() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let now = Local::now().timestamp_millis();
        let day = 86_400_000;
        add_sized(&h, &t.0, "old.png", 100, now - 90 * day, false);
        let m = add_sized(&h, &t.0, "mid.png", 100, now - 5 * day, false);
        let n = add_sized(&h, &t.0, "new.png", 100, now - day, false);
        assert_eq!(h.apply_retention(30, 150).unwrap(), 2);
        assert!(h.get(m).unwrap().is_none());
        assert!(h.get(n).unwrap().is_some());
    }

    #[test]
    fn rescan_adds_and_removes() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        write_png(&t.0, "one.png", 12, 8);
        write_png(&t.0, "Two.PNG", 5, 5);
        std::fs::write(t.0.join("clip.mp4"), b"mp4").unwrap();
        std::fs::write(t.0.join("anim.gif"), b"GIF89a\x07\x00\x09\x00\x00\x00\x00").unwrap();
        std::fs::write(t.0.join("notes.txt"), b"x").unwrap();
        std::fs::write(t.0.join(".hidden.png"), b"x").unwrap();
        std::fs::create_dir_all(t.0.join("sub")).unwrap();
        write_png(&t.0.join("sub"), "nested.png", 2, 2);
        write_png(&t.0.join(".stinkshot"), "inside.png", 2, 2);
        assert_eq!(h.rescan().unwrap(), (4, 0));
        assert_eq!(h.rescan().unwrap(), (0, 0));
        let one = h.find_by_path(&t.0.join("one.png")).unwrap().unwrap();
        assert_eq!((one.width, one.height, one.kind), (12, 8, ItemKind::Image));
        let two = h.find_by_path(&t.0.join("Two.PNG")).unwrap().unwrap();
        assert_eq!(two.kind, ItemKind::Image);
        let clip = h.find_by_path(&t.0.join("clip.mp4")).unwrap().unwrap();
        assert_eq!(clip.kind, ItemKind::Video);
        let gif = h.find_by_path(&t.0.join("anim.gif")).unwrap().unwrap();
        assert_eq!((gif.kind, gif.width, gif.height), (ItemKind::Gif, 7, 9));
        h.set_starred(one.id, true).unwrap();
        h.rescan().unwrap();
        assert!(h.get(one.id).unwrap().unwrap().starred);

        std::fs::remove_file(t.0.join("clip.mp4")).unwrap();
        write_png(&t.0, "three.png", 3, 3);
        assert_eq!(h.rescan().unwrap(), (1, 1));
        assert!(h.find_by_path(&t.0.join("clip.mp4")).unwrap().is_none());
        assert_eq!(h.stats().unwrap().count, 4);
    }

    #[test]
    fn thumbnails_generate_cache_and_refresh() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let p = write_png(&t.0, "big.png", 400, 200);
        let id = h.add(new_item(p, "", "")).unwrap();
        let th = h.thumbnail(id, 100).unwrap().unwrap();
        assert_eq!(
            th,
            t.0.join(".stinkshot")
                .join("thumbs")
                .join(format!("{id}.png"))
        );
        assert_eq!(read_dimensions(&th), (100, 50));
        h.thumbnail(id, 200).unwrap().unwrap();
        assert_eq!(read_dimensions(&th), (200, 100));
        h.thumbnail(id, 50).unwrap().unwrap();
        assert_eq!(read_dimensions(&th), (200, 100));
        let small = write_png(&t.0, "small.png", 10, 10);
        let sid = h.add(new_item(small, "", "")).unwrap();
        let st = h.thumbnail(sid, 300).unwrap().unwrap();
        assert_eq!(read_dimensions(&st), (10, 10));
        h.touch(id, 400, 200).unwrap();
        assert!(!th.exists());
        assert!(h.thumbnail(12345, 64).unwrap().is_none());
    }

    #[test]
    fn stars_survive_a_corrupt_index_and_retention_spares_them() {
        let t = Tmp::new();
        let p = write_png(&t.0, "keep.png", 4, 4);
        let other = write_png(&t.0, "other.png", 4, 4);
        // Both were edited in the app, which is what a rebuilt index knows them by.
        give_sidecar(&p);
        give_sidecar(&other);
        {
            let h = History::open(&t.0).unwrap();
            let id = h.add(new_item(p.clone(), "", "")).unwrap();
            h.set_starred(id, true).unwrap();
            h.thumbnail(id, 32).unwrap().unwrap();
        }
        let db = t.0.join(".stinkshot").join("index.sqlite");
        std::fs::write(&db, b"this is not a database at all, not even close").unwrap();
        let h = History::open(&t.0).unwrap();
        assert!(!t.0.join(".stinkshot").join("thumbs").exists());
        assert_eq!(h.rescan().unwrap().0, 2);
        assert!(h.find_by_path(&p).unwrap().unwrap().starred);
        assert!(
            !h.find_by_path(&t.0.join("other.png"))
                .unwrap()
                .unwrap()
                .starred
        );
        // Everything is "old", but the star keeps the file through retention.
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.apply_retention(0, 1).unwrap();
        assert!(p.exists());
        assert!(!t.0.join("other.png").exists());
    }

    #[test]
    fn retention_spares_files_it_did_not_create() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let own = write_png(&t.0, "own.png", 4, 4);
        h.add(new_item(own.clone(), "", "")).unwrap();
        // Dropped into the folder by something else.
        let photo = write_png(&t.0, "photo.png", 4, 4);
        assert_eq!(h.rescan().unwrap(), (1, 0));
        assert_eq!(h.find_by_path(&photo).unwrap().unwrap().mode, EXTERNAL_MODE);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(h.apply_retention(0, 1).unwrap(), 1);
        assert!(!own.exists());
        assert!(photo.exists());
    }

    #[test]
    fn an_index_that_cannot_be_opened_is_left_alone() {
        let t = Tmp::new();
        let meta = t.0.join(".stinkshot");
        // Not a damaged file: something that cannot be opened at all.
        std::fs::create_dir_all(meta.join("index.sqlite")).unwrap();
        assert!(History::open(&t.0).is_err());
        assert!(meta.join("index.sqlite").is_dir());
        assert!(!meta.join("index.sqlite.corrupt").exists());
    }

    #[test]
    fn retention_counts_sidecars_and_thumbnails() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let a = write_png(&t.0, "a.png", 40, 40);
        let b = write_png(&t.0, "b.png", 40, 40);
        let ia = h.add(new_item(a.clone(), "", "")).unwrap();
        h.add(new_item(b.clone(), "", "")).unwrap();
        let flat = std::fs::metadata(&a).unwrap().len();
        let base = stinkshot_core::Rgba::new(40, 40);
        let mut doc = stinkshot_core::annotate::Doc::new(40, 40);
        doc.add(
            stinkshot_core::annotate::ShapeKind::Line {
                from: stinkshot_core::geom::PtF { x: 0.0, y: 0.0 },
                to: stinkshot_core::geom::PtF { x: 5.0, y: 5.0 },
            },
            stinkshot_core::annotate::Style::default(),
        );
        stinkshot_core::annotate::save_editable(&a, &base, &doc).unwrap();
        // Flat files alone (2 * flat) fit this cap; with a's sidecar they do not.
        let removed = h.apply_retention(0, flat * 2 + 1).unwrap();
        assert_eq!(removed, 1);
        assert!(h.get(ia).unwrap().is_none());
    }

    #[test]
    fn video_thumbnail_only_when_stored() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let v = t.0.join("v.mp4");
        std::fs::write(&v, b"x").unwrap();
        let id = h.add(new_item(v, "", "")).unwrap();
        assert!(h.thumbnail(id, 64).unwrap().is_none());
        let mut px = stinkshot_core::Rgba::new(8, 6);
        px.data.iter_mut().for_each(|b| *b = 200);
        h.store_thumbnail(id, &px).unwrap();
        let th = h.thumbnail(id, 64).unwrap().unwrap();
        assert_eq!(read_dimensions(&th), (8, 6));
        let bad = stinkshot_core::Rgba {
            width: 4,
            height: 4,
            data: vec![0; 3],
        };
        assert!(h.store_thumbnail(id, &bad).is_err());
    }

    #[test]
    fn fts_expression_shapes() {
        assert_eq!(fts_expression("invo"), Some("\"invo\"*".into()));
        assert_eq!(fts_expression("a b"), Some("\"a\" \"b\"*".into()));
        assert_eq!(
            fts_expression("it's-ok"),
            Some("\"it\" \"s\" \"ok\"*".into())
        );
        assert_eq!(fts_expression("   "), None);
    }

    #[test]
    fn concurrent_use() {
        let t = Tmp::new();
        let h = std::sync::Arc::new(History::open(&t.0).unwrap());
        let mut joins = vec![];
        for i in 0..4 {
            let h = h.clone();
            let dir = t.0.clone();
            joins.push(std::thread::spawn(move || {
                for j in 0..5 {
                    let p = write_png(&dir, &format!("t{i}_{j}.png"), 3, 3);
                    h.add(new_item(p, "app", "")).unwrap();
                    h.query(&Query::default()).unwrap();
                }
            }));
        }
        for j in joins {
            j.join().unwrap();
        }
        assert_eq!(h.stats().unwrap().count, 20);
    }

    /// An edit sidecar like the app writes for an annotated capture.
    fn give_sidecar(path: &Path) {
        let base = stinkshot_core::Rgba::new(4, 4);
        let mut doc = stinkshot_core::annotate::Doc::new(4, 4);
        doc.add(
            stinkshot_core::annotate::ShapeKind::Line {
                from: stinkshot_core::geom::PtF { x: 0.0, y: 0.0 },
                to: stinkshot_core::geom::PtF { x: 3.0, y: 3.0 },
            },
            stinkshot_core::annotate::Style::default(),
        );
        stinkshot_core::annotate::save_editable(path, &base, &doc).unwrap();
    }

    fn all(h: &History) -> Vec<ItemMeta> {
        h.query(&Query::default()).unwrap()
    }

    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let dest = to.join(e.file_name());
            if e.path().is_dir() {
                copy_tree(&e.path(), &dest);
            } else {
                std::fs::copy(e.path(), dest).unwrap();
            }
        }
    }

    #[test]
    fn a_rebuilt_index_does_not_take_over_the_users_own_files() {
        let t = Tmp::new();
        // The folder is shared with other things: a photo and a clip the user put there.
        let photo = write_png(&t.0, "holiday.png", 4, 4);
        let clip = t.0.join("birthday.mp4");
        std::fs::write(&clip, vec![0u8; 5000]).unwrap();
        // And one capture the app made and edited.
        let own = write_png(&t.0, "Stinkshot_1.png", 4, 4);
        give_sidecar(&own);
        std::fs::create_dir_all(t.0.join(".stinkshot")).unwrap();
        std::fs::write(
            t.0.join(".stinkshot").join("index.sqlite"),
            vec![0x42u8; 4096],
        )
        .unwrap();
        let h = History::open(&t.0).unwrap();
        assert_eq!(h.rescan().unwrap(), (3, 0));
        assert_eq!(h.find_by_path(&photo).unwrap().unwrap().mode, EXTERNAL_MODE);
        assert_eq!(h.find_by_path(&clip).unwrap().unwrap().mode, EXTERNAL_MODE);
        assert_eq!(h.find_by_path(&own).unwrap().unwrap().mode, "");
        // Everything is years old and far over the cap.
        for m in all(&h) {
            set_created(&h, m.id, 1000);
        }
        assert_eq!(h.apply_retention(1, 1).unwrap(), 1);
        assert!(photo.exists());
        assert!(clip.exists());
        assert!(!own.exists());
        // A later rescan (files dropped in afterwards) never adopts.
        let later = write_png(&t.0, "later.png", 4, 4);
        give_sidecar(&later);
        h.rescan().unwrap();
        assert_eq!(h.find_by_path(&later).unwrap().unwrap().mode, EXTERNAL_MODE);
    }

    #[test]
    fn the_size_cap_does_not_count_files_it_cannot_remove() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let a = add_sized(&h, &t.0, "a.png", 100, 1000, false);
        let b = add_sized(&h, &t.0, "b.png", 100, 2000, false);
        // Far bigger than the cap, and not this app's.
        std::fs::write(t.0.join("movie.mp4"), vec![0u8; 50_000]).unwrap();
        assert_eq!(h.rescan().unwrap(), (1, 0));
        // Own captures use 200 of 250: nothing to do.
        assert_eq!(h.apply_retention(0, 250).unwrap(), 0);
        assert!(h.get(a).unwrap().is_some());
        assert!(h.get(b).unwrap().is_some());
        // Over the cap on their own: oldest goes, the external file stays.
        assert_eq!(h.apply_retention(0, 150).unwrap(), 1);
        assert!(h.get(a).unwrap().is_none());
        assert!(h.get(b).unwrap().is_some());
        assert!(t.0.join("movie.mp4").exists());
    }

    #[test]
    fn a_moved_folder_keeps_its_metadata() {
        let outer = Tmp::new();
        let before = outer.0.join("Shots");
        let after = outer.0.join("Screenshots");
        std::fs::create_dir_all(&before).unwrap();
        let id = {
            let h = History::open(&before).unwrap();
            let p = write_png(&before, "a.png", 6, 4);
            let id = h.add(new_item(p, "firefox", "Invoice")).unwrap();
            h.set_starred(id, true).unwrap();
            h.set_ocr_text(id, "total due").unwrap();
            id
        };
        std::fs::rename(&before, &after).unwrap();
        let h = History::open(&after).unwrap();
        // Until the rescan the row names a file that is not in this folder: not handed out.
        assert!(h.get(id).unwrap().is_none());
        assert_eq!(h.rescan().unwrap(), (0, 0));
        let m = h.get(id).unwrap().unwrap();
        assert_eq!(m.path, after.join("a.png"));
        assert_eq!(
            (m.app.as_str(), m.mode.as_str(), m.starred),
            ("firefox", "region", true)
        );
        assert_eq!(m.ocr_text.as_deref(), Some("total due"));
        assert_eq!(h.query(&text_query("due")).unwrap().len(), 1);
        assert_eq!(all(&h).len(), 1);
    }

    #[test]
    fn rows_from_another_folder_are_never_deleted_or_handed_out() {
        let outer = Tmp::new();
        let original = outer.0.join("Original");
        let copy = outer.0.join("Copy");
        std::fs::create_dir_all(&original).unwrap();
        let (kept, only_there) = {
            let h = History::open(&original).unwrap();
            let kept = h
                .add(new_item(write_png(&original, "kept.png", 4, 4), "a", ""))
                .unwrap();
            let only = h
                .add(new_item(write_png(&original, "only.png", 4, 4), "b", ""))
                .unwrap();
            for id in [kept, only] {
                set_created(&h, id, 1000);
            }
            (kept, only)
        };
        // The user copies the folder and points the app at the copy; one file did not
        // come along.
        copy_tree(&original, &copy);
        std::fs::remove_file(copy.join("only.png")).unwrap();
        let h = History::open(&copy).unwrap();
        // The copied index still names the files of the original folder.
        assert!(all(&h).is_empty());
        assert!(h.get(kept).unwrap().is_none());
        assert!(h.thumbnail(kept, 32).unwrap().is_none());
        assert!(h.pending_ocr(0).unwrap().is_empty());
        h.delete(kept).unwrap();
        assert_eq!(h.apply_retention(1, 1).unwrap(), 0);
        assert!(original.join("kept.png").exists());
        assert!(original.join("only.png").exists());
        // The rescan points the row at the file here, and forgets the one that is not.
        assert_eq!(h.rescan().unwrap(), (0, 1));
        assert_eq!(h.get(kept).unwrap().unwrap().path, copy.join("kept.png"));
        assert!(h.get(only_there).unwrap().is_none());
        assert_eq!(h.stats().unwrap().count, 1);
        // Now retention works on the copy, and only on the copy.
        assert_eq!(h.apply_retention(1, 0).unwrap(), 1);
        assert!(!copy.join("kept.png").exists());
        assert!(original.join("kept.png").exists());
        assert!(original.join("only.png").exists());
    }

    #[test]
    fn a_wildcard_path_in_the_index_is_inert() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let real = write_png(&t.0, "real.png", 4, 4);
        let id = h.add(new_item(real.clone(), "", "")).unwrap();
        // A damaged or crafted row: the shell would expand this to every PNG here.
        h.lock()
            .execute(
                "INSERT INTO items (path, name, kind, created) VALUES (?1, '*.png', 0, 1000)",
                [t.0.join("*.png").to_string_lossy().as_ref()],
            )
            .unwrap();
        let bad = h.lock().last_insert_rowid();
        assert!(!h.owns(&t.0.join("*.png")));
        assert!(!h.owns(&t.0.join("wh?t.png")));
        assert!(!h.owns(&t.0.join("sub").join("x.png")));
        assert!(!h.owns(&t.0));
        assert!(h.owns(&PathBuf::from(real.to_string_lossy().to_uppercase())));
        assert!(h.get(bad).unwrap().is_none());
        assert_eq!(all(&h).len(), 1);
        h.delete(bad).unwrap();
        assert_eq!(h.apply_retention(1, 0).unwrap(), 0);
        assert!(real.exists());
        assert_eq!(h.rescan().unwrap(), (0, 1));
        assert!(h.get(id).unwrap().is_some());
        assert_eq!(h.stats().unwrap().count, 1);
    }

    #[test]
    fn adding_over_a_known_path_starts_the_item_afresh() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let p = write_png(&t.0, "shot.png", 8, 8);
        let id = h.add(new_item(p.clone(), "old", "old title")).unwrap();
        h.set_starred(id, true).unwrap();
        set_created(&h, id, 1000);
        // The file is replaced by a new capture under the same name.
        write_png(&t.0, "shot.png", 10, 6);
        assert_eq!(h.add(new_item(p, "new", "")).unwrap(), id);
        let m = h.get(id).unwrap().unwrap();
        assert!(!m.starred);
        assert!(m.created.timestamp_millis() > Local::now().timestamp_millis() - 60_000);
        assert_eq!((m.width, m.height, m.app.as_str()), (10, 6, "new"));
        assert!(h.saved_stars().is_empty());
    }

    #[test]
    fn deleted_text_does_not_stay_in_the_index_files() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let on: i64 = h
            .lock()
            .query_row("PRAGMA secure_delete", [], |r| r.get(0))
            .unwrap();
        assert_eq!(on, 1);
        let p = write_png(&t.0, "a.png", 4, 4);
        let id = h
            .add(new_item(p, "bank", "Zyxwvut Account 99887766"))
            .unwrap();
        h.set_ocr_text(id, "qqsecretphraseqq").unwrap();
        h.delete(id).unwrap();
        let meta = t.0.join(".stinkshot");
        let wal = std::fs::metadata(meta.join("index.sqlite-wal")).map(|m| m.len());
        assert_eq!(wal.unwrap_or(0), 0, "log emptied after a delete");
        let db = std::fs::read(meta.join("index.sqlite")).unwrap();
        let has = |needle: &[u8]| db.windows(needle.len()).any(|w| w == needle);
        assert!(!has(b"qqsecretphraseqq"));
        assert!(!has(b"99887766"));
    }

    #[test]
    fn retention_preview_matches_apply_and_deletes_nothing() {
        let t = Tmp::new();
        let h = History::open(&t.0).unwrap();
        let now = Local::now().timestamp_millis();
        let day = 86_400_000;
        add_sized(&h, &t.0, "old.png", 100, now - 90 * day, false);
        add_sized(&h, &t.0, "olds.png", 100, now - 90 * day, true);
        add_sized(&h, &t.0, "mid.png", 100, now - 5 * day, false);
        add_sized(&h, &t.0, "new.png", 100, now - day, false);
        assert_eq!(h.retention_preview(0, 0).unwrap(), (0, 0));
        assert_eq!(h.retention_preview(30, 0).unwrap(), (1, 100));
        // Age removes old; 300 left with a 250 cap: mid goes too (the starred one stays).
        assert_eq!(h.retention_preview(30, 250).unwrap(), (2, 200));
        assert_eq!(h.retention_preview(0, 250).unwrap(), (2, 200));
        assert_eq!(h.stats().unwrap().count, 4);
        assert!(t.0.join("old.png").exists());
        assert_eq!(h.apply_retention(30, 250).unwrap(), 2);
        assert_eq!(h.retention_preview(30, 250).unwrap(), (0, 0));
    }
}
