//! File naming. Tokens: {yyyy} {MM} {dd} {HH} {mm} {ss} {fff} (millis) {app} (source
//! process name without .exe, sanitized) {title} (window title, sanitized, max 40 chars)
//! {w} {h}. Characters invalid in Windows filenames are replaced with '_'.

use std::path::{Path, PathBuf};

pub struct NameParts<'a> {
    pub now: chrono::DateTime<chrono::Local>,
    pub app: &'a str,
    pub title: &'a str,
    pub width: u32,
    pub height: u32,
}

/// Longest file stem (without extension or uniqueness suffix) we will produce.
const MAX_STEM_CHARS: usize = 150;
const MAX_TITLE_CHARS: usize = 40;
const FALLBACK_STEM: &str = "Stinkshot";

pub fn expand_pattern(pattern: &str, parts: &NameParts) -> String {
    use chrono::Timelike;
    let now = parts.now;
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut rest = pattern;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            // Unterminated brace: keep the rest literally.
            out.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let token = &after[..close];
        match token {
            "yyyy" => out.push_str(&format!("{:04}", chrono::Datelike::year(&now))),
            "MM" => out.push_str(&format!("{:02}", chrono::Datelike::month(&now))),
            "dd" => out.push_str(&format!("{:02}", chrono::Datelike::day(&now))),
            "HH" => out.push_str(&format!("{:02}", now.hour())),
            "mm" => out.push_str(&format!("{:02}", now.minute())),
            "ss" => out.push_str(&format!("{:02}", now.second())),
            "fff" => out.push_str(&format!("{:03}", now.nanosecond() / 1_000_000 % 1000)),
            "app" => out.push_str(&sanitize(strip_exe(parts.app))),
            "title" => {
                let t: String = sanitize(parts.title)
                    .chars()
                    .take(MAX_TITLE_CHARS)
                    .collect();
                out.push_str(t.trim_end());
            }
            "w" => out.push_str(&parts.width.to_string()),
            "h" => out.push_str(&parts.height.to_string()),
            _ => {
                // Unknown token: keep it visible rather than silently dropping it.
                out.push('{');
                out.push_str(token);
                out.push('}');
            }
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    finalize_stem(&out)
}

/// Names `unique_path` has handed out whose files are not written yet (lowercased paths).
/// The caller writes the file after this returns, often a while after (an export), so
/// "does not exist" alone lets two saves in the same second pick the same name and one
/// overwrite the other.
static RESERVED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// `folder/<expanded>.<ext>`, appending " (2)", " (3)"... if the file exists or the name
/// was already given to another caller in this process. Choosing and reserving happen under
/// one lock.
pub fn unique_path(folder: &Path, pattern: &str, parts: &NameParts, ext: &str) -> PathBuf {
    let stem = expand_pattern(pattern, parts);
    let ext = ext.trim_start_matches('.');
    let make = |s: &str| -> PathBuf {
        if ext.is_empty() {
            folder.join(s)
        } else {
            folder.join(format!("{s}.{ext}"))
        }
    };
    let key = |p: &Path| p.to_string_lossy().to_lowercase();
    let mut reserved = RESERVED.lock().unwrap_or_else(|e| e.into_inner());
    // A reserved name whose file now exists is covered by the check on disk. One whose
    // save failed stays until the process ends, which only costs that name.
    reserved.retain(|p| !Path::new(p).exists());
    let candidates =
        std::iter::once(make(&stem)).chain((2..10_000u32).map(|n| make(&format!("{stem} ({n})"))));
    for p in candidates {
        let k = key(&p);
        if !p.exists() && !reserved.contains(&k) {
            reserved.push(k);
            return p;
        }
    }
    // Pathological: fall back to a nanosecond-stamped name.
    make(&format!(
        "{stem} ({})",
        parts.now.timestamp_nanos_opt().unwrap_or(0)
    ))
}

/// Replace characters that are invalid in Windows file names (`<>:"/\|?*` and control
/// characters) with `_`, and trim surrounding whitespace and trailing dots. May return an
/// empty string.
pub fn sanitize(s: &str) -> String {
    let replaced: String = s
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    replaced.trim().trim_end_matches(['.', ' ']).to_string()
}

fn strip_exe(app: &str) -> &str {
    let t = app.trim();
    match t.len().checked_sub(4) {
        Some(i) if t.is_char_boundary(i) && t[i..].eq_ignore_ascii_case(".exe") => &t[..i],
        _ => t,
    }
}

/// Final pass on an expanded name: sanitize, length-cap, avoid reserved device names.
fn finalize_stem(s: &str) -> String {
    let mut stem = sanitize(s);
    if stem.chars().count() > MAX_STEM_CHARS {
        stem = stem.chars().take(MAX_STEM_CHARS).collect();
        stem = stem.trim_end_matches(['.', ' ']).to_string();
    }
    if stem.is_empty() {
        return FALLBACK_STEM.to_string();
    }
    // Windows decides on the part before the first dot, ignoring spaces after it:
    // "CON.txt - Notepad" is the console as much as "CON" is.
    let base = stem
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    let reserved = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && base.as_bytes()[3].is_ascii_digit()
            && base.as_bytes()[3] != b'0');
    if reserved {
        // Right after the device name, so that part itself is no longer one. `base` is
        // ASCII here, so its length is a char boundary in `stem`.
        stem.insert(base.len(), '_');
    }
    stem
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn parts<'a>(app: &'a str, title: &'a str) -> NameParts<'a> {
        let now = chrono::Local
            .with_ymd_and_hms(2026, 11, 6, 9, 5, 7)
            .unwrap()
            + chrono::Duration::milliseconds(42);
        NameParts {
            now,
            app,
            title,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn default_pattern_expands() {
        let cfg = crate::config::Capture::default();
        assert_eq!(
            expand_pattern(&cfg.filename_pattern, &parts("", "")),
            "Stinkshot_2026-11-06_09-05-07"
        );
    }

    #[test]
    fn all_tokens() {
        let p = parts("notepad.exe", "Doc");
        assert_eq!(
            expand_pattern(
                "{yyyy}{MM}{dd}-{HH}{mm}{ss}.{fff}_{app}_{title}_{w}x{h}",
                &p
            ),
            "20261106-090507.042_notepad_Doc_1920x1080"
        );
    }

    #[test]
    fn app_exe_stripped_case_insensitive() {
        assert_eq!(expand_pattern("{app}", &parts("Code.EXE", "")), "Code");
        assert_eq!(expand_pattern("{app}", &parts("chrome", "")), "chrome");
    }

    #[test]
    fn title_sanitized_and_truncated() {
        let long = "a/b:c*d?".repeat(20);
        let out = expand_pattern("{title}", &parts("", &long));
        assert_eq!(out.chars().count(), 40);
        assert!(!out.contains(['/', ':', '*', '?']));
        assert!(out.starts_with("a_b_c_d_"));
    }

    #[test]
    fn invalid_chars_in_pattern_replaced() {
        assert_eq!(expand_pattern("a<b>c|d", &parts("", "")), "a_b_c_d");
        assert_eq!(expand_pattern("x/y\\z", &parts("", "")), "x_y_z");
    }

    #[test]
    fn unknown_and_unterminated_tokens_stay_literal() {
        assert_eq!(
            expand_pattern("{nope}-{yyyy", &parts("", "")),
            "{nope}-{yyyy"
        );
    }

    #[test]
    fn empty_and_reserved_names_are_fixed() {
        assert_eq!(expand_pattern("", &parts("", "")), "Stinkshot");
        assert_eq!(expand_pattern("{title}", &parts("", "")), "Stinkshot");
        assert_eq!(expand_pattern("con", &parts("", "")), "con_");
        assert_eq!(expand_pattern("COM1", &parts("", "")), "COM1_");
        assert_eq!(expand_pattern("COM0", &parts("", "")), "COM0");
        assert_eq!(expand_pattern("shot...", &parts("", "")), "shot");
        // A dot after the device name does not make it a file: the underscore goes into
        // the name itself.
        assert_eq!(
            expand_pattern("{title}", &parts("", "CON.txt - Notepad")),
            "CON_.txt - Notepad"
        );
        assert_eq!(expand_pattern("nul .2026", &parts("", "")), "nul_ .2026");
        assert_eq!(expand_pattern("lpt9.a.b", &parts("", "")), "lpt9_.a.b");
        assert_eq!(expand_pattern("console.txt", &parts("", "")), "console.txt");
    }

    #[test]
    fn two_saves_in_the_same_second_get_different_names() {
        let dir = std::env::temp_dir().join(format!("stinkshot-core-race-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = parts("", "");
        // Neither file is written yet when the second name is asked for.
        let a = unique_path(&dir, "same", &p, "png");
        let b = unique_path(&dir, "same", &p, "png");
        assert_eq!(a, dir.join("same.png"));
        assert_eq!(b, dir.join("same (2).png"));
        // From several threads at once: every name is different.
        let names: Vec<PathBuf> = std::thread::scope(|s| {
            let hs: Vec<_> = (0..8)
                .map(|_| s.spawn(|| unique_path(&dir, "burst", &parts("", ""), "png")))
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len());
        // Once a file exists, the name on disk is what counts.
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        assert_eq!(
            unique_path(&dir, "same", &p, "png"),
            dir.join("same (3).png")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sanitize_basics() {
        assert_eq!(sanitize("  a:b\tc.  "), "a_b_c");
        assert_eq!(sanitize(""), "");
        assert_eq!(sanitize("ok name"), "ok name");
    }

    #[test]
    fn unicode_title_is_safe() {
        let out = expand_pattern("{title}", &parts("", "日本語のタイトル – ñ"));
        assert_eq!(out, "日本語のタイトル – ñ");
    }

    #[test]
    fn unique_path_appends_counter() {
        let dir = std::env::temp_dir().join(format!("stinkshot-core-name-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = parts("", "");
        let first = unique_path(&dir, "shot", &p, "png");
        assert_eq!(first, dir.join("shot.png"));
        std::fs::write(&first, b"x").unwrap();
        let second = unique_path(&dir, "shot", &p, "png");
        assert_eq!(second, dir.join("shot (2).png"));
        std::fs::write(&second, b"x").unwrap();
        assert_eq!(
            unique_path(&dir, "shot", &p, ".png"),
            dir.join("shot (3).png")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
