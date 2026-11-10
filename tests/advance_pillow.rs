use std::path::Path;

use slot_cart_studio::advance::pillow::{alpha_composite, fit, resize_lanczos, round_half_even};
use slot_cart_studio::advance::Image;

fn png(path: &Path) -> Image {
    let mut decoder = png::Decoder::new(std::fs::File::open(path).unwrap());
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    assert_eq!(info.color_type, png::ColorType::Rgba);
    Image {
        w: info.width,
        h: info.height,
        px: buf,
    }
}

fn dump(name: &str) -> Image {
    png(&Path::new("tests/fixtures/advance/expected/pillow").join(format!("{name}.png")))
}

fn board() -> Image {
    png(Path::new("assets/advance/board.png"))
}

#[test]
fn resize_matches_pillow() {
    let src = dump("tinted-GBA-35353a-solid");
    let out = resize_lanczos(&src, 120, 71, [0.0, 0.0, src.w as f64, src.h as f64]);
    let want = dump("resize-gba");
    assert_eq!((out.w, out.h), (want.w, want.h));
    assert!(out.px == want.px);
}

#[test]
fn fit_matches_pillow_both_sizes() {
    let logo = png(Path::new("tests/fixtures/advance/card/Labels/GBA/Logo.png"));
    for (w, h) in [(88, 45), (74, 66)] {
        let out = fit(&logo, w, h);
        let want = dump(&format!("fit-{w}x{h}"));
        assert_eq!((out.w, out.h), (w, h));
        assert!(out.px == want.px, "fit {w}x{h}");
    }
}

#[test]
fn composite_matches_pillow() {
    let mut dst = dump("tinted-GBA-c2332e-clear");
    alpha_composite(&mut dst, &board(), 0, 0);
    assert!(dst.px == dump("composite").px);
}

#[test]
fn half_even() {
    assert_eq!(round_half_even(0.5), 0);
    assert_eq!(round_half_even(1.5), 2);
    assert_eq!(round_half_even(2.5), 2);
    assert_eq!(round_half_even(16.67), 17);
}
