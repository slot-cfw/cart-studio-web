use super::Image;

const TRANSPARENT: u16 = 0x7c1f;

fn five(c: u8) -> u16 {
    (f64::from(c) * 31.0 / 255.0 + 0.5) as u16
}

pub fn bmp555(img: &Image) -> Vec<u8> {
    let (w, h) = (img.w as usize, img.h as usize);
    let stride = (w * 2 + 3) & !3;
    let mut data = vec![0u8; stride * h];
    for y in 0..h {
        for x in 0..w {
            let [r, g, b, a] = img.at(x as u32, y as u32);
            let mut v = TRANSPARENT;
            if a >= 128 {
                v = five(b) | five(g) << 5 | five(r) << 10;
                if v == TRANSPARENT {
                    v ^= 1;
                }
            }
            let at = (h - 1 - y) * stride + x * 2;
            data[at..at + 2].copy_from_slice(&v.to_le_bytes());
        }
    }
    let mut out = Vec::with_capacity(54 + data.len());
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(54 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&data);
    out
}
