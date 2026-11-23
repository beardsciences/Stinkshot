//! Writes the Microsoft Store package images (MSIX `Assets\`) from the same logo renderer as
//! the exe icon. Used by `scripts/store-package.ps1`.
//!
//! `cargo run -p stinkshot --example store_assets -- <out-dir>`

use std::path::Path;

/// Square images: (file name, canvas size, logo size). The logo is centred on a transparent
/// canvas; tiles leave a margin as Windows' own tiles do.
const SQUARE: &[(&str, u32, u32)] = &[
    ("StoreLogo.png", 50, 50),
    ("Square44x44Logo.png", 44, 44),
    ("Square150x150Logo.png", 150, 96),
    // Taskbar, Start and Alt+Tab pick these exact sizes instead of scaling.
    ("Square44x44Logo.targetsize-16.png", 16, 16),
    ("Square44x44Logo.targetsize-24.png", 24, 24),
    ("Square44x44Logo.targetsize-32.png", 32, 32),
    ("Square44x44Logo.targetsize-48.png", 48, 48),
    ("Square44x44Logo.targetsize-256.png", 256, 256),
    ("Square44x44Logo.altform-unplated_targetsize-16.png", 16, 16),
    ("Square44x44Logo.altform-unplated_targetsize-24.png", 24, 24),
    ("Square44x44Logo.altform-unplated_targetsize-32.png", 32, 32),
    ("Square44x44Logo.altform-unplated_targetsize-48.png", 48, 48),
    (
        "Square44x44Logo.altform-unplated_targetsize-256.png",
        256,
        256,
    ),
];

fn main() -> anyhow::Result<()> {
    let out = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: store_assets <out-dir>"))?;
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;
    for (name, canvas, logo) in SQUARE {
        write(out, name, *canvas, *canvas, *logo)?;
    }
    write(out, "Wide310x150Logo.png", 310, 150, 96)?;
    println!("wrote {} images to {}", SQUARE.len() + 1, out.display());
    Ok(())
}

/// The logo at `logo` px, centred on a transparent `w` x `h` canvas.
fn write(dir: &Path, name: &str, w: u32, h: u32, logo: u32) -> anyhow::Result<()> {
    let icon = stinkshot_logo::render_icon(logo);
    let mut canvas = vec![0u8; (w * h * 4) as usize];
    let (ox, oy) = ((w - logo) / 2, (h - logo) / 2);
    for y in 0..logo {
        let src = &icon[(y * logo * 4) as usize..((y + 1) * logo * 4) as usize];
        let at = (((oy + y) * w + ox) * 4) as usize;
        canvas[at..at + src.len()].copy_from_slice(src);
    }
    image::save_buffer(dir.join(name), &canvas, w, h, image::ColorType::Rgba8)?;
    Ok(())
}
