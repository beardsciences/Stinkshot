//! OCR behind a swappable trait. Default engine: Windows.Media.Ocr (offline, uses the
//! user's installed language packs; `TryCreateFromUserProfileLanguages`). Small images are
//! upscaled 2x before recognition (UI text at 100% scale OCRs poorly otherwise); images
//! larger than `OcrEngine.MaxImageDimension` are tiled vertically with overlap and merged.

use stinkshot_core::{PxRect, Rgba};
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine as WinOcrEngine;
use windows::Storage::Streams::DataWriter;

pub use crate::portable::ocr::*;

/// Implemented on this OS (the stand-ins say false).
pub const SUPPORTED: bool = true;

pub struct WindowsOcr {
    engine: WinOcrEngine,
    /// One OcrEngine instance rejects overlapping RecognizeAsync calls, so serialize them.
    gate: std::sync::Mutex<()>,
}

// SAFETY: Windows.Media.Ocr.OcrEngine is an agile (free-threaded) runtime class; it can be
// used from any thread with an initialized apartment, which `recognize` ensures.
unsafe impl Send for WindowsOcr {}
// SAFETY: see above; the engine holds no thread-affine state.
unsafe impl Sync for WindowsOcr {}

impl WindowsOcr {
    /// Errors if no OCR language is installed.
    pub fn new() -> anyhow::Result<WindowsOcr> {
        crate::util::ensure_com();
        if let Ok(engine) = WinOcrEngine::TryCreateFromUserProfileLanguages() {
            return Ok(WindowsOcr {
                engine,
                gate: std::sync::Mutex::new(()),
            });
        }
        // Profile languages have no OCR pack; use any installed recognizer language.
        if let Ok(langs) = WinOcrEngine::AvailableRecognizerLanguages() {
            for lang in langs {
                if let Ok(engine) = WinOcrEngine::TryCreateFromLanguage(&lang) {
                    return Ok(WindowsOcr {
                        engine,
                        gate: std::sync::Mutex::new(()),
                    });
                }
            }
        }
        anyhow::bail!(
            "no OCR language is installed (Settings > Time & language > Language & region > add a language with OCR support)"
        )
    }

    fn recognize_tile(&self, w: u32, h: u32, bgra: &[u8]) -> anyhow::Result<Vec<TileLine>> {
        let writer = DataWriter::new()?;
        writer.WriteBytes(bgra)?;
        let buffer = writer.DetachBuffer()?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            w as i32,
            h as i32,
        )?;
        let _gate = self.gate.lock().unwrap_or_else(|p| p.into_inner());
        let result = self.engine.RecognizeAsync(&bitmap)?.join()?;
        let mut out = Vec::new();
        for line in result.Lines()? {
            let text = line.Text()?.to_string();
            let mut bounds: Option<[f32; 4]> = None; // x0, y0, x1, y1
            let mut words = Vec::new();
            for word in line.Words()? {
                let r = word.BoundingRect()?;
                let b = [r.X, r.Y, r.X + r.Width, r.Y + r.Height];
                words.push((word.Text()?.to_string(), b));
                bounds = Some(match bounds {
                    None => b,
                    Some(o) => [
                        o[0].min(b[0]),
                        o[1].min(b[1]),
                        o[2].max(b[2]),
                        o[3].max(b[3]),
                    ],
                });
            }
            if let Some(b) = bounds {
                out.push((text, b, words));
            }
        }
        Ok(out)
    }
}

impl OcrEngine for WindowsOcr {
    fn name(&self) -> &str {
        "Windows OCR"
    }

    fn recognize(&self, img: &Rgba) -> anyhow::Result<OcrResult> {
        crate::util::ensure_com();
        if img.width == 0 || img.height == 0 {
            return Ok(OcrResult::default());
        }
        let max_dim = WinOcrEngine::MaxImageDimension().unwrap_or(10000).max(256);
        let scale = choose_scale(img.width, img.height, max_dim);
        let nw = ((img.width as f32 * scale).round() as u32).max(1);
        let nh = ((img.height as f32 * scale).round() as u32).max(1);
        let scaled = if nw == img.width && nh == img.height {
            img.clone()
        } else {
            resize(img, nw, nh)
        };
        let tile_h = TILE_TARGET.min(max_dim);
        let spans = tile_spans(scaled.height, tile_h, TILE_OVERLAP);
        let last = spans.len() - 1;
        let mut lines: Vec<OcrLine> = Vec::new();
        for (i, (y0, len)) in spans.iter().copied().enumerate() {
            let tile = if spans.len() == 1 {
                scaled.clone()
            } else {
                scaled.crop(PxRect::new(0, y0 as i32, scaled.width as i32, len as i32))
            };
            let (tile, tw, th) = pad_min(&tile);
            let found = self.recognize_tile(tw, th, &to_opaque_bgra(&tile))?;
            // In overlap zones only the tile whose interior contains the line's centre keeps it.
            let keep_from = if i == 0 {
                f32::MIN
            } else {
                (y0 + TILE_OVERLAP / 2) as f32
            };
            let keep_to = if i == last {
                f32::MAX
            } else {
                (y0 + len - TILE_OVERLAP / 2) as f32
            };
            // Tile px (x0, y0, x1, y1) -> input image px.
            let to_px = |b: [f32; 4]| {
                let x0 = (b[0] / scale).floor() as i32;
                let y0f = ((y0 as f32 + b[1]) / scale).floor() as i32;
                let x1 = (b[2] / scale).ceil() as i32;
                let y1 = ((y0 as f32 + b[3]) / scale).ceil() as i32;
                PxRect::new(x0.max(0), y0f.max(0), (x1 - x0).max(0), (y1 - y0f).max(0))
                    .intersect(&img.bounds())
                    .unwrap_or_default()
            };
            for (text, b, ws) in found {
                let cy = y0 as f32 + (b[1] + b[3]) / 2.0;
                if cy < keep_from || cy >= keep_to {
                    continue;
                }
                let words = ws
                    .into_iter()
                    .map(|(text, wb)| OcrWord {
                        text,
                        rect: to_px(wb),
                    })
                    .collect();
                lines.push(OcrLine {
                    text,
                    rect: to_px(b),
                    words,
                });
            }
        }
        let text = lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        Ok(OcrResult { text, lines })
    }
}
