use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use xcap::image::{imageops, DynamicImage, GrayImage, RgbaImage};
use xcap::Monitor;

const REFERENCE_HEIGHT: f64 = 1440.0;

/// Capture box relative to the map crosshair, in px at 1440p.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRegion {
    pub left: u32,
    pub right: u32,
    pub up: u32,
    pub down: u32,
    pub threshold: u8,
    pub upscale: u32,
}

impl Default for CaptureRegion {
    fn default() -> Self {
        Self {
            left: 24,
            right: 200,
            up: 130,
            down: 12,
            threshold: 200,
            upscale: 3,
        }
    }
}

pub struct Capture {
    pub raw: RgbaImage,
    pub variants: Vec<Variant>,
}

pub fn cursor_pos() -> Result<(i32, i32)> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p) }.context("GetCursorPos failed")?;
    Ok((p.x, p.y))
}

pub fn capture_at_cursor(region: &CaptureRegion) -> Result<Capture> {
    let (cx, cy) = cursor_pos()?;
    let monitor = Monitor::from_point(cx, cy).map_err(|e| anyhow!("no monitor at cursor: {e}"))?;

    let mx = monitor.x().map_err(|e| anyhow!("{e}"))?;
    let my = monitor.y().map_err(|e| anyhow!("{e}"))?;
    let mw = monitor.width().map_err(|e| anyhow!("{e}"))?;
    let mh = monitor.height().map_err(|e| anyhow!("{e}"))?;
    let scale = mh as f64 / REFERENCE_HEIGHT;

    let (lx, ly) = (cx - mx, cy - my);
    let (x0, y0, w, h) = region_box(lx, ly, mw, mh, region, scale);
    let raw = monitor
        .capture_region(x0, y0, w, h)
        .map_err(|e| anyhow!("screen capture failed: {e}"))?;

    Ok(Capture {
        variants: preprocess(&raw, region, (lx - x0 as i32, ly - y0 as i32), scale),
        raw,
    })
}

/// `scale` is screen size relative to 1440p.
pub fn capture_from_image(screen: &RgbaImage, cx: i32, cy: i32, region: &CaptureRegion, scale: f64) -> Capture {
    let (x0, y0, w, h) = region_box(cx, cy, screen.width(), screen.height(), region, scale);
    let raw = imageops::crop_imm(screen, x0, y0, w, h).to_image();
    Capture {
        variants: preprocess(&raw, region, (cx - x0 as i32, cy - y0 as i32), scale),
        raw,
    }
}

fn region_box(lx: i32, ly: i32, mw: u32, mh: u32, region: &CaptureRegion, scale: f64) -> (u32, u32, u32, u32) {
    let (mw, mh) = (mw as i32, mh as i32);
    let px = |v: u32| (v as f64 * scale).round() as i32;

    let x0 = (lx - px(region.left)).clamp(0, mw - 1);
    let y0 = (ly - px(region.up)).clamp(0, mh - 1);
    let x1 = (lx + px(region.right)).clamp(x0 + 1, mw);
    let y1 = (ly + px(region.down)).clamp(y0 + 1, mh);
    (x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32)
}

// Windows OCR is unreliable on large glyphs, so lines are normalised to these heights.
const LINE_HEIGHTS: [u32; 3] = [48, 36, 64];

/// Where the game draws the "y" and "x" readout lines, relative to the
/// crosshair, in px at 1440p: (dx0, dy0, dx1, dy1).
const STRIPS: [(i32, i32, i32, i32); 2] = [(2, -112, 190, -72), (2, -34, 190, -2)];

/// Largest glyph at 1440p; bigger blobs are map icons, brackets or lines.
const MAX_GLYPH_H: f64 = 22.0;
const MAX_GLYPH_W: f64 = 16.0;

fn preprocess(raw: &RgbaImage, region: &CaptureRegion, cursor: (i32, i32), scale: f64) -> Vec<Variant> {
    let mut variants = Vec::new();
    for threshold in [region.threshold, region.threshold.saturating_sub(35)] {
        let lines: Vec<GrayImage> = STRIPS
            .iter()
            .filter_map(|strip| readout_line(raw, cursor, *strip, threshold, scale))
            .collect();
        if lines.is_empty() {
            continue;
        }
        for height in LINE_HEIGHTS {
            variants.push(
                lines
                    .iter()
                    .map(|l| line_image(l, (0, 0, l.width(), l.height()), height))
                    .collect(),
            );
        }
    }
    variants.extend(generic_variants(raw, region));
    variants
}

/// White, unsaturated glyph-sized blobs inside one readout strip, drawn dark on
/// light and cropped to their extent. Coloured zones, tinted icons and anything
/// icon-sized or cut by the strip edge are left out.
fn readout_line(raw: &RgbaImage, (cx, cy): (i32, i32), strip: (i32, i32, i32, i32), threshold: u8, scale: f64) -> Option<GrayImage> {
    let (w, h) = (raw.width() as i32, raw.height() as i32);
    let px = |v: i32| (v as f64 * scale).round() as i32;
    let x0 = (cx + px(strip.0)).clamp(0, w);
    let y0 = (cy + px(strip.1)).clamp(0, h);
    let x1 = (cx + px(strip.2)).clamp(x0, w);
    let y1 = (cy + px(strip.3)).clamp(y0, h);
    let (sw, sh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    if sw < 4 || sh < 4 {
        return None;
    }
    let pixel = |x: usize, y: usize| raw.get_pixel((x0 as usize + x) as u32, (y0 as usize + y) as u32).0;

    let white: Vec<bool> = (0..sh * sw)
        .map(|i| {
            let p = pixel(i % sw, i / sw);
            let (lo, hi) = (p[0].min(p[1]).min(p[2]), p[0].max(p[1]).max(p[2]));
            lo >= threshold && hi - lo <= 48
        })
        .collect();

    let max_h = (MAX_GLYPH_H * scale).ceil() as usize;
    let max_w = (MAX_GLYPH_W * scale).ceil() as usize;
    let mut keep = vec![false; sh * sw];
    let mut seen = vec![false; sh * sw];
    for start in 0..white.len() {
        if !white[start] || seen[start] {
            continue;
        }
        let mut blob = vec![start];
        seen[start] = true;
        let mut i = 0;
        while i < blob.len() {
            let (bx, by) = ((blob[i] % sw) as i32, (blob[i] / sw) as i32);
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                let (nx, ny) = (bx + dx, by + dy);
                if nx < 0 || ny < 0 || nx >= sw as i32 || ny >= sh as i32 {
                    continue;
                }
                let n = ny as usize * sw + nx as usize;
                if white[n] && !seen[n] {
                    seen[n] = true;
                    blob.push(n);
                }
            }
            i += 1;
        }

        let (mut bx0, mut bx1, mut by0, mut by1) = (sw, 0, sh, 0);
        for &n in &blob {
            let (x, y) = (n % sw, n / sw);
            bx0 = bx0.min(x);
            bx1 = bx1.max(x);
            by0 = by0.min(y);
            by1 = by1.max(y);
        }
        let glyph_sized = by1 - by0 < max_h && bx1 - bx0 < max_w;
        let inside = by0 > 0 && by1 < sh - 1;
        if glyph_sized && inside {
            for &n in &blob {
                keep[n] = true;
            }
        }
    }

    let kept = |x: usize, y: usize| keep[y * sw + x];
    let xs: Vec<usize> = (0..sw).filter(|&x| (0..sh).any(|y| kept(x, y))).collect();
    let ys: Vec<usize> = (0..sh).filter(|&y| (0..sw).any(|x| kept(x, y))).collect();
    let (&kx0, &kx1, &ky0, &ky1) = (xs.first()?, xs.last()?, ys.first()?, ys.last()?);

    // Shade pixels next to a kept blob by brightness to keep anti-aliased edges.
    let (ox, oy) = (kx0.saturating_sub(1), ky0.saturating_sub(1));
    let (ow, oh) = ((kx1 + 2).min(sw) - ox, (ky1 + 2).min(sh) - oy);
    let mut out = GrayImage::from_pixel(ow as u32, oh as u32, xcap::image::Luma([255]));
    for y in 0..oh {
        for x in 0..ow {
            let (sx, sy) = (ox + x, oy + y);
            let near = (sy.saturating_sub(1)..=(sy + 1).min(sh - 1))
                .any(|ny| (sx.saturating_sub(1)..=(sx + 1).min(sw - 1)).any(|nx| kept(nx, ny)));
            if near {
                let p = pixel(sx, sy);
                let luma = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
                out.put_pixel(x as u32, y as u32, xcap::image::Luma([255 - luma.min(255) as u8]));
            }
        }
    }
    Some(out)
}

/// Fallback when the readout is not where expected: binarise the whole box and
/// split it into text lines.
fn generic_variants(raw: &RgbaImage, region: &CaptureRegion) -> Vec<Variant> {
    let gray = DynamicImage::ImageRgba8(raw.clone()).to_luma8();
    let factor = region.upscale.max(1);
    let big = imageops::resize(
        &gray,
        gray.width() * factor,
        gray.height() * factor,
        imageops::FilterType::CatmullRom,
    );

    let mut binary = big.clone();
    for p in binary.pixels_mut() {
        p.0[0] = if p.0[0] >= region.threshold { 0 } else { 255 };
    }
    erase_long_lines(&mut binary);

    let mut inverted = big;
    imageops::invert(&mut inverted);

    let boxes = text_lines(&binary, 2 * factor);
    let mut variants = Vec::new();
    for height in LINE_HEIGHTS {
        for img in [&binary, &inverted] {
            variants.push(boxes.iter().map(|b| line_image(img, *b, height)).collect());
        }
    }
    variants
}

fn line_image(img: &GrayImage, (x0, y0, x1, y1): Bbox, height: u32) -> GrayImage {
    let piece = imageops::crop_imm(img, x0, y0, x1 - x0, y1 - y0).to_image();
    let width = ((x1 - x0) as f64 * height as f64 / (y1 - y0) as f64).round().max(1.0) as u32;
    let piece = imageops::resize(&piece, width, height, imageops::FilterType::Triangle);

    let pad = height * 3 / 4;
    let mut out = GrayImage::from_pixel(width + 2 * pad, height + 2 * pad, xcap::image::Luma([255]));
    imageops::replace(&mut out, &piece, pad as i64, pad as i64);
    out
}

/// Removes crosshair and grid lines.
fn erase_long_lines(binary: &mut GrayImage) {
    let (w, h) = binary.dimensions();
    let ink = |img: &GrayImage, x: u32, y: u32| img.get_pixel(x, y).0[0] == 0;

    let rows: Vec<u32> = (0..h).filter(|&y| (0..w).filter(|&x| ink(binary, x, y)).count() * 10 > w as usize * 6).collect();
    let cols: Vec<u32> = (0..w).filter(|&x| (0..h).filter(|&y| ink(binary, x, y)).count() * 10 > h as usize * 6).collect();

    for y in rows {
        for x in 0..w {
            binary.put_pixel(x, y, xcap::image::Luma([255]));
        }
    }
    for x in cols {
        for y in 0..h {
            binary.put_pixel(x, y, xcap::image::Luma([255]));
        }
    }
}

pub type Variant = Vec<GrayImage>;

// (x0, y0, x1, y1)
type Bbox = (u32, u32, u32, u32);

fn text_lines(binary: &GrayImage, max_gap: u32) -> Vec<Bbox> {
    let (w, h) = binary.dimensions();
    let ink = |x: u32, y: u32| binary.get_pixel(x, y).0[0] == 0;
    let row_has_ink = |y: u32| (0..w).any(|x| ink(x, y));

    let mut bands: Vec<(u32, u32)> = Vec::new();
    let mut last_ink: Option<u32> = None;
    for y in 0..h {
        if !row_has_ink(y) {
            continue;
        }
        match (last_ink, bands.last_mut()) {
            (Some(prev), Some(band)) if y - prev <= max_gap => band.1 = y + 1,
            _ => bands.push((y, y + 1)),
        }
        last_ink = Some(y);
    }

    bands
        .into_iter()
        .filter(|(y0, y1)| y1 - y0 >= 4)
        .filter_map(|(y0, y1)| {
            let cols: Vec<u32> = (0..w).filter(|&x| (y0..y1).any(|y| ink(x, y))).collect();
            Some((*cols.first()?, y0, cols.last()? + 1, y1))
        })
        .collect()
}
