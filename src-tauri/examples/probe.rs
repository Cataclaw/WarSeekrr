// cargo run --example probe -- <screenshot.png> <crosshair_x> <crosshair_y> [out_dir] [scale]

use warseekrr_lib::capture::{capture_from_image, CaptureRegion};
use warseekrr_lib::recognize_capture;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: probe <screenshot.png> <crosshair_x> <crosshair_y> [out_dir] [scale]");
        std::process::exit(2);
    }
    let screen = xcap::image::open(&args[1]).expect("open screenshot").to_rgba8();
    let cx: i32 = args[2].parse().expect("crosshair_x");
    let cy: i32 = args[3].parse().expect("crosshair_y");
    let out = std::path::PathBuf::from(args.get(4).map(String::as_str).unwrap_or("probe-out"));
    std::fs::create_dir_all(&out).unwrap();

    let scale: f64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(screen.width() as f64 / 2560.0);
    let cap = capture_from_image(&screen, cx, cy, &CaptureRegion::default(), scale);
    cap.raw.save(out.join("raw.png")).unwrap();
    for (i, v) in cap.variants.iter().enumerate() {
        for (j, line) in v.iter().enumerate() {
            line.save(out.join(format!("variant{i}-line{j}.png"))).unwrap();
        }
    }

    let (coord, text) = recognize_capture(&cap);
    println!("--- ocr text ---\n{text}\n--- parsed ---\n{coord:?}");
}
