use warseekrr_lib::ballistics::Coord;
use warseekrr_lib::capture::{capture_from_image, CaptureRegion};
use warseekrr_lib::recognize_capture;
use xcap::image::imageops::FilterType;

// Fixtures are 1440p screenshots scaled to 2000px wide, cropped to the map.
const SCALE: f64 = 2000.0 / 2560.0;

fn open(file: &str) -> xcap::image::DynamicImage {
    xcap::image::open(format!("{}/tests/fixtures/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn read(file: &str, cx: i32, cy: i32) -> Option<Coord> {
    let screen = open(file).to_rgba8();
    recognize_capture(&capture_from_image(&screen, cx, cy, &CaptureRegion::default(), SCALE)).0
}

#[test]
fn zoomed_out_map() {
    assert_eq!(read("zoomed-out_x74.16_y60.85.png", 343, 545), Some(Coord { x: 74.16, y: 60.85 }));
}

#[test]
fn zoomed_in_map_with_grid() {
    assert_eq!(read("zoomed-in_x79.28_y74.32.png", 282, 491), Some(Coord { x: 79.28, y: 74.32 }));
}

#[test]
fn close_zoom_near_player_icons() {
    assert_eq!(read("zoomed-in_x98.53_y111.17.png", 382, 369), Some(Coord { x: 98.53, y: 111.17 }));
}

#[test]
fn native_1440p_glyph_size() {
    let img = open("zoomed-in_x98.53_y111.17.png");
    let (w, h) = ((img.width() as f64 / SCALE) as u32, (img.height() as f64 / SCALE) as u32);
    let screen = img.resize_exact(w, h, FilterType::Lanczos3).to_rgba8();
    let (cx, cy) = ((382.0 / SCALE) as i32, (369.0 / SCALE) as i32);
    let cap = capture_from_image(&screen, cx, cy, &CaptureRegion::default(), 1.0);
    assert_eq!(recognize_capture(&cap).0, Some(Coord { x: 98.53, y: 111.17 }));
}

#[test]
fn readout_over_icons_and_tooltip() {
    assert_eq!(read("icons-tooltip_x67.53_y106.71.png", 381, 297), Some(Coord { x: 67.53, y: 106.71 }));
}

#[test]
fn readout_over_fob_icon_and_zone() {
    assert_eq!(read("fob-zone_x68.57_y103.98.png", 349, 346), Some(Coord { x: 68.57, y: 103.98 }));
}
