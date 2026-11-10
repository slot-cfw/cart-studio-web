#![allow(dead_code)]

use std::path::Path;

use slot_cart_studio::advance::{decode, Image};

pub const FIX: &str = "tests/fixtures/advance";
pub const JPEG: &str =
    "zune-jpeg is not libjpeg-turbo, so decoded JPEG pixels differ slightly from Pillow's";
pub const TOLERATED: &[(&str, &str)] = &[
    ("SYSTEM/SLOT/CARTS/GBA/Photo.bmp", JPEG),
    ("SYSTEM/SLOT/CARTS_FLAT/GBA/Photo.bmp", JPEG),
    ("SYSTEM/SLOT/LABELS/GBA/Photo.bmp", JPEG),
];

pub fn dump(name: &str) -> Image {
    let path = Path::new(FIX)
        .join("expected/pillow")
        .join(format!("{name}.png"));
    let (px, w, h) = decode::rgba(&std::fs::read(path).unwrap()).unwrap();
    Image { w, h, px }
}

pub fn rgb555(bmp: &[u8]) -> Vec<[u8; 3]> {
    bmp[54..]
        .chunks_exact(2)
        .map(|p| {
            let v = u16::from_le_bytes([p[0], p[1]]);
            [(v >> 10 & 31) as u8, (v >> 5 & 31) as u8, (v & 31) as u8]
        })
        .collect()
}

pub fn max_step_diff(a: &[u8], b: &[u8]) -> u8 {
    rgb555(a)
        .iter()
        .zip(rgb555(b))
        .flat_map(|(x, y)| (0..3).map(move |i| x[i].abs_diff(y[i])))
        .max()
        .unwrap_or(0)
}

pub fn same_shape(a: &[u8], b: &[u8]) -> bool {
    let key = |bmp: &[u8]| {
        bmp[54..]
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]) == 0x7c1f)
            .collect::<Vec<_>>()
    };
    a.len() == b.len() && a[..54] == b[..54] && key(a) == key(b)
}

pub fn check(path: &str, got: &[u8], want: &[u8]) {
    if got == want {
        return;
    }
    assert!(
        same_shape(got, want),
        "{path}: header, size or transparency differs"
    );
    let diff = max_step_diff(got, want);
    let allowed = TOLERATED.iter().any(|(p, _)| *p == path);
    assert!(
        allowed && diff <= 1,
        "{path}: differs by up to {diff} RGB555 steps"
    );
}
