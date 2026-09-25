use anyhow::{Context, Result};
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use xcap::image::GrayImage;

pub struct Ocr {
    engine: OcrEngine,
}

impl Ocr {
    pub fn new() -> Result<Self> {
        // S_FALSE / RPC_E_CHANGED_MODE are fine.
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };

        let engine = OcrEngine::TryCreateFromUserProfileLanguages()
            .or_else(|_| {
                OcrEngine::TryCreateFromLanguage(&Language::CreateLanguage(&HSTRING::from("en-US"))?)
            })
            .context("no Windows OCR language pack installed (Settings > Time & language > Language)")?;

        Ok(Self { engine })
    }

    pub fn recognize(&self, image: &GrayImage) -> Result<String> {
        let (w, h) = image.dimensions();
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for p in image.pixels() {
            let v = p.0[0];
            bgra.extend_from_slice(&[v, v, v, 255]);
        }

        let writer = DataWriter::new()?;
        writer.WriteBytes(&bgra)?;
        let buffer = writer.DetachBuffer()?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, w as i32, h as i32)?;

        let result = self.engine.RecognizeAsync(&bitmap)?.join()?;

        let lines = result.Lines()?;
        let mut out = Vec::new();
        for line in lines {
            out.push(line.Text()?.to_string());
        }
        Ok(out.join("\n"))
    }
}
