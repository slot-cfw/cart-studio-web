pub mod pillow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Image {
    pub fn new(w: u32, h: u32) -> Image {
        Image {
            w,
            h,
            px: vec![0; (w * h * 4) as usize],
        }
    }

    pub fn filled(w: u32, h: u32, rgba: [u8; 4]) -> Image {
        Image {
            w,
            h,
            px: rgba.repeat((w * h) as usize),
        }
    }

    pub fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.w + x) * 4) as usize;
        [self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]]
    }

    pub fn set(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let i = ((y * self.w + x) * 4) as usize;
        self.px[i..i + 4].copy_from_slice(&rgba);
    }

    pub fn crop(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> Image {
        let mut out = Image::new(x1 - x0, y1 - y0);
        for y in y0..y1 {
            for x in x0..x1 {
                if x < self.w && y < self.h {
                    out.set(x - x0, y - y0, self.at(x, y));
                }
            }
        }
        out
    }
}
