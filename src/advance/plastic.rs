use std::cell::RefCell;
use std::collections::HashMap;

use super::assets;
use super::shell::{Finish, Shape};
use super::Image;

type Rgb = [u32; 3];

fn overlay(base: u8, colour: u8) -> u8 {
    let (base, colour) = (u32::from(base), u32::from(colour));
    if base >= 128 {
        (colour + ((255 - colour) * (base - 128) + 63) / 127).min(255) as u8
    } else {
        colour.saturating_sub((colour * (128 - base) + 64) / 128) as u8
    }
}

fn sparkle(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9e3779b9) ^ y.wrapping_mul(0x85ebca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^ (h >> 12)
}

type TintKey = (String, [u8; 3], bool, bool);
type FaceKey = (Shape, [u8; 3], Finish, [u32; 4]);

thread_local! {
    static TINTED: RefCell<HashMap<TintKey, Image>> = RefCell::new(HashMap::new());
    static FACES: RefCell<HashMap<FaceKey, Image>> = RefCell::new(HashMap::new());
}

pub fn tinted(name: &str, colour: [u8; 3], clear: bool, glitter: bool) -> Image {
    let key = (name.to_string(), colour, clear, glitter);
    if let Some(hit) = TINTED.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let mut im = assets::png(name);
    let w = im.w;
    for (i, p) in im.px.chunks_exact_mut(4).enumerate() {
        let (x, y) = (i as u32 % w, i as u32 / w);
        for c in 0..3 {
            p[c] = overlay(p[c], colour[c]);
        }
        if clear {
            p[3] = ((u32::from(p[3]) * 204 + 127) / 255) as u8;
        }
        if glitter && p[3] != 0 && sparkle(x, y).is_multiple_of(100) {
            for c in &mut p[..3] {
                *c = ((u32::from(*c) * 155 + 255 * 100) / 255) as u8;
            }
            p[3] = 255;
        }
    }
    TINTED.with(|c| c.borrow_mut().insert(key, im.clone()));
    im
}

fn lerp(a: Rgb, b: Rgb, n: u32, d: u32) -> Rgb {
    [0, 1, 2].map(|i| (a[i] * (d - n) + b[i] * n) / d)
}

pub fn shell_face(shape: Shape, colour: [u8; 3], finish: Finish, panel: [u32; 4]) -> Image {
    let key = (shape, colour, finish, panel);
    if let Some(hit) = FACES.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let name = match shape {
        Shape::Gba => "gba_simple_mould.png",
        Shape::Gb => "gb_simple_mould.png",
        Shape::Gbc => "gbc_simple_mould.png",
    };
    let mould = assets::png(name);
    let (w, h) = (mould.w, mould.h);
    let plastic: Rgb = colour.map(u32::from);
    let clear = finish != Finish::Solid;
    let through = |behind: Rgb| {
        lerp(
            [0, 1, 2].map(|i| plastic[i] * behind[i] / 255),
            plastic.map(|a| a * a / 255),
            150,
            255,
        )
    };
    let base = through(plastic);
    let mean = (base[0] + base[1] + base[2]) / 3;
    let edge = base.map(|c| {
        let v = (3 * c + mean) / 4;
        v + (255 - v) * 2 / 5
    });
    let rim = if shape == Shape::Gba {
        (4 * w + 120) / 240
    } else {
        (7 * w + 120) / 240
    };
    let (left, right, top, c0, c1, contacts, traces) = if shape == Shape::Gba {
        (0.135, 0.875, 50.0 / 212.0, 0.17, 0.84, 0.85, 0.76)
    } else {
        (0.075, 0.925, 0.07, 0.1, 0.9, 0.905, 0.84)
    };
    let mut out: Vec<[u32; 4]> = Vec::with_capacity((w * h) as usize);
    for (i, m) in mould.px.chunks_exact(4).enumerate() {
        let (shadow, light, depth, alpha) = (
            u32::from(m[0]),
            u32::from(m[1]),
            u32::from(m[2]),
            u32::from(m[3]),
        );
        let (x, y) = (i as u32 % w, i as u32 / w);
        let (fx, fy) = (f64::from(x) / f64::from(w), f64::from(y) / f64::from(h));
        let mut c = plastic;
        if clear {
            let mut inside = plastic;
            if left <= fx && fx < right && fy >= top {
                let pitch = (c1 - c0) / 32.0;
                let phase = (fx - c0) % pitch;
                let pin = c0 <= fx && fx < c1 && phase < pitch * 0.6;
                inside = if pin && fy >= contacts {
                    [230, 180, 70]
                } else if pin && phase < pitch * 0.2 && fy >= traces {
                    [90, 180, 116]
                } else {
                    [44, 150, 82]
                };
            }
            c = lerp(edge, through(inside), depth.min(rim), rim);
            if finish == Finish::Glitter && sparkle(x, y).is_multiple_of(100) {
                c = lerp(c, [255, 255, 255], 100, 255);
            }
        }
        let under = c;
        if shadow != 0 {
            c = [0, 1, 2].map(|k| {
                ((under[k] as f64 * 0.62) as u32 * shadow + c[k] * (255 - shadow) + 127) / 255
            });
        }
        if light != 0 {
            c = [0, 1, 2].map(|k| {
                let v = under[k];
                ((v + ((255 - v) as f64 * 0.24) as u32) * light + c[k] * (255 - light) + 127) / 255
            });
        }
        out.push([c[0], c[1], c[2], alpha]);
    }
    let [lx, ly, lw, lh] = panel.map(|v| v as i64);
    let bevel = ((3 * w + 120) / 240) as i64;
    let (x0, y0, x1, y1) = (lx - bevel, ly - bevel, lx + lw + bevel, ly + lh + bevel);
    for y in y0..y1 {
        for x in x0..x1 {
            if lx <= x && x < lx + lw && ly <= y && y < ly + lh {
                continue;
            }
            let upper = (y - y0).min(x - x0) <= (y1 - y - 1).min(x1 - x - 1);
            let at = (y * i64::from(w) + x) as usize;
            let old = out[at];
            let c = if clear {
                [0, 1, 2]
                    .map(|k| ((old[k] as f64 * if upper { 0.7 } else { 1.18 }) as u32).min(255))
            } else {
                [0, 1, 2].map(|k| {
                    ((plastic[k] as f64 * if upper { 0.55 } else { 1.45 }) as u32).min(255)
                })
            };
            out[at] = [c[0], c[1], c[2], old[3]];
        }
    }
    let im = Image {
        w,
        h,
        px: out.iter().flat_map(|p| p.map(|v| v as u8)).collect(),
    };
    FACES.with(|c| c.borrow_mut().insert(key, im.clone()));
    im
}
