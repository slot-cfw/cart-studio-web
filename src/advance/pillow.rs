#![allow(clippy::needless_range_loop)]

use std::f64::consts::PI;

use super::Image;

const PRECISION_BITS: u32 = 32 - 8 - 2;
const SUPPORT: f64 = 3.0;

fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        return 1.0;
    }
    let x = x * PI;
    x.sin() / x
}

fn lanczos(x: f64) -> f64 {
    if (-3.0..3.0).contains(&x) {
        sinc(x) * sinc(x / 3.0)
    } else {
        0.0
    }
}

fn shift_div255(a: u32) -> u32 {
    ((a >> 8) + a) >> 8
}

fn muldiv255(a: u8, b: u8) -> u8 {
    shift_div255(u32::from(a) * u32::from(b) + 128) as u8
}

fn clip8(v: i32) -> u8 {
    (v >> PRECISION_BITS).clamp(0, 255) as u8
}

struct Coeffs {
    ksize: usize,
    bounds: Vec<(usize, usize)>,
    kk: Vec<i32>,
}

fn precompute(in_size: u32, in0: f32, in1: f32, out_size: u32) -> Coeffs {
    let scale = f64::from(in1 - in0) / f64::from(out_size);
    let filterscale = scale.max(1.0);
    let support = SUPPORT * filterscale;
    let ksize = support.ceil() as usize * 2 + 1;
    let mut bounds = Vec::with_capacity(out_size as usize);
    let mut kk = vec![0i32; out_size as usize * ksize];
    for xx in 0..out_size as usize {
        let center = f64::from(in0) + (xx as f64 + 0.5) * scale;
        let ss = 1.0 / filterscale;
        let xmin = ((center - support + 0.5) as i32).max(0);
        let xmax = ((center + support + 0.5) as i32).min(in_size as i32) - xmin;
        let mut k = vec![0.0f64; ksize];
        let mut ww = 0.0;
        for x in 0..xmax.max(0) as usize {
            let w = lanczos((x as f64 + xmin as f64 - center + 0.5) * ss);
            k[x] = w;
            ww += w;
        }
        for x in 0..xmax.max(0) as usize {
            if ww != 0.0 {
                k[x] /= ww;
            }
        }
        for (x, v) in k.iter().enumerate() {
            let scaled = v * f64::from(1u32 << PRECISION_BITS);
            kk[xx * ksize + x] = if *v < 0.0 {
                (-0.5 + scaled) as i32
            } else {
                (0.5 + scaled) as i32
            };
        }
        bounds.push((xmin as usize, xmax.max(0) as usize));
    }
    Coeffs { ksize, bounds, kk }
}

fn horizontal(src: &Image, c: &Coeffs, w: u32, y0: usize, rows: usize) -> Image {
    let mut out = Image::new(w, rows as u32);
    let stride = src.w as usize * 4;
    for yy in 0..rows {
        let row = &src.px[(yy + y0) * stride..(yy + y0 + 1) * stride];
        for xx in 0..w as usize {
            let (xmin, xmax) = c.bounds[xx];
            let k = &c.kk[xx * c.ksize..];
            let mut ss = [1i32 << (PRECISION_BITS - 1); 4];
            for x in 0..xmax {
                for (ch, s) in ss.iter_mut().enumerate() {
                    *s += i32::from(row[(x + xmin) * 4 + ch]) * k[x];
                }
            }
            let i = (yy * w as usize + xx) * 4;
            for ch in 0..4 {
                out.px[i + ch] = clip8(ss[ch]);
            }
        }
    }
    out
}

fn vertical(src: &Image, c: &Coeffs, h: u32) -> Image {
    let mut out = Image::new(src.w, h);
    let stride = src.w as usize * 4;
    for yy in 0..h as usize {
        let (ymin, ymax) = c.bounds[yy];
        let k = &c.kk[yy * c.ksize..];
        for xx in 0..src.w as usize {
            let mut ss = [1i32 << (PRECISION_BITS - 1); 4];
            for y in 0..ymax {
                for (ch, s) in ss.iter_mut().enumerate() {
                    *s += i32::from(src.px[(y + ymin) * stride + xx * 4 + ch]) * k[y];
                }
            }
            let i = (yy * src.w as usize + xx) * 4;
            for ch in 0..4 {
                out.px[i + ch] = clip8(ss[ch]);
            }
        }
    }
    out
}

fn resample(img: &Image, w: u32, h: u32, b: [f32; 4]) -> Image {
    let need_h = w != img.w || b[0] != 0.0 || b[2] != w as f32;
    let need_v = h != img.h || b[1] != 0.0 || b[3] != h as f32;
    let ch = precompute(img.w, b[0], b[2], w);
    let mut cv = precompute(img.h, b[1], b[3], h);
    let first = cv.bounds[0].0;
    let last = cv.bounds[h as usize - 1].0 + cv.bounds[h as usize - 1].1;
    let mut cur = None;
    if need_h {
        for bound in cv.bounds.iter_mut() {
            bound.0 -= first;
        }
        cur = Some(horizontal(img, &ch, w, first, last - first));
    }
    if need_v {
        let src = cur.as_ref().unwrap_or(img);
        cur = Some(vertical(src, &cv, h));
    }
    cur.unwrap_or_else(|| img.clone())
}

fn premultiply(img: &Image) -> Image {
    let mut out = img.clone();
    for p in out.px.chunks_exact_mut(4) {
        let a = p[3];
        for c in &mut p[..3] {
            *c = muldiv255(*c, a);
        }
    }
    out
}

fn unpremultiply(img: &mut Image) {
    for p in img.px.chunks_exact_mut(4) {
        let a = u32::from(p[3]);
        if a != 0 && a != 255 {
            for c in &mut p[..3] {
                *c = (255 * u32::from(*c) / a).min(255) as u8;
            }
        }
    }
}

pub fn resize_lanczos(img: &Image, w: u32, h: u32, bx: [f64; 4]) -> Image {
    if (img.w, img.h) == (w, h) && bx == [0.0, 0.0, f64::from(w), f64::from(h)] {
        return img.clone();
    }
    let pre = premultiply(img);
    let b = bx.map(|v| v as f32);
    let mut out = if b[0] - b[0].trunc() == 0.0
        && b[2] - b[0] == w as f32
        && b[1] - b[1].trunc() == 0.0
        && b[3] - b[1] == h as f32
    {
        pre.crop(b[0] as u32, b[1] as u32, b[2] as u32, b[3] as u32)
    } else {
        resample(&pre, w, h, b)
    };
    unpremultiply(&mut out);
    out
}

pub fn fit(img: &Image, w: u32, h: u32) -> Image {
    let (lw, lh) = (f64::from(img.w), f64::from(img.h));
    let live = lw / lh;
    let output = f64::from(w) / f64::from(h);
    let (cw, ch) = if live == output {
        (lw, lh)
    } else if live >= output {
        (output * lh, lh)
    } else {
        (lw, lw / output)
    };
    let left = (lw - cw) * 0.5;
    let top = (lh - ch) * 0.5;
    resize_lanczos(img, w, h, [left, top, left + cw, top + ch])
}

pub fn alpha_composite(dst: &mut Image, src: &Image, x0: u32, y0: u32) {
    const P: u32 = 7;
    for sy in 0..src.h {
        for sx in 0..src.w {
            let (x, y) = (x0 + sx, y0 + sy);
            if x >= dst.w || y >= dst.h {
                continue;
            }
            let s = src.at(sx, sy).map(u32::from);
            if s[3] == 0 {
                continue;
            }
            let d = dst.at(x, y).map(u32::from);
            let blend = d[3] * (255 - s[3]);
            let outa255 = s[3] * 255 + blend;
            let coef1 = s[3] * 255 * 255 * (1 << P) / outa255;
            let coef2 = 255 * (1 << P) - coef1;
            let ch =
                |i: usize| (shift_div255(s[i] * coef1 + d[i] * coef2 + (0x80 << P)) >> P) as u8;
            dst.set(
                x,
                y,
                [ch(0), ch(1), ch(2), shift_div255(outa255 + 0x80) as u8],
            );
        }
    }
}

pub fn round_half_even(v: f64) -> i64 {
    v.round_ties_even() as i64
}
