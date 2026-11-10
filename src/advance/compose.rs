use super::decode::Label;
use super::pillow::{alpha_composite, fit, resize_lanczos, round_half_even};
use super::plastic::{shell_face, tinted};
use super::shell::{shape_for, Finish, Shape, Shell};
use super::text::generated_label;
use super::{assets, Image, SPECS};

const PAPER: [u8; 4] = [0xe9, 0xe4, 0xd8, 255];

fn prefix(shape: Shape) -> &'static str {
    match shape {
        Shape::Gba => "gba",
        Shape::Gb => "gb",
        Shape::Gbc => "gbc",
    }
}

fn third(v: u32) -> u32 {
    round_half_even(f64::from(v) / 3.0) as u32
}

pub fn label_size(shape: Shape) -> (u32, u32) {
    let panel = SPECS[shape as usize].1;
    (third(panel[2]), third(panel[3]))
}

fn mask(paper: &mut Image) {
    for p in paper.px.chunks_exact_mut(4) {
        p[3] = 255;
    }
    let (w, h) = (paper.w, paper.h);
    for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        let mut px = paper.at(x, y);
        px[3] = 0;
        paper.set(x, y, px);
    }
}

pub fn compose(
    platform: &str,
    title: &str,
    label: Option<&Label>,
    shell: &Shell,
    skeuomorphic: bool,
) -> (Image, Image) {
    let shape = shape_for(platform, shell);
    let (_, panel, small) = SPECS[shape as usize];
    let clear = shell.finish != Finish::Solid;
    let glitter = shell.finish == Finish::Glitter;
    let mut face = if skeuomorphic {
        let kind = if clear && shape != Shape::Gb {
            "clear"
        } else {
            "neutral"
        };
        tinted(
            &format!("{}_{kind}.png", prefix(shape)),
            shell.colour,
            clear,
            glitter,
        )
    } else {
        shell_face(shape, shell.colour, shell.finish, panel)
    };
    if clear && skeuomorphic {
        let mut back = tinted(
            &format!("{}_interior.png", prefix(shape)),
            shell.colour,
            false,
            false,
        );
        let board = assets::png(if shape == Shape::Gba {
            "board.png"
        } else {
            "gb_board.png"
        });
        alpha_composite(&mut back, &board, 0, 0);
        alpha_composite(&mut back, &face, 0, 0);
        face = back;
    }
    let mut face = resize_lanczos(
        &face,
        small.0,
        small.1,
        [0.0, 0.0, f64::from(face.w), f64::from(face.h)],
    );
    let (fw, fh) = (third(panel[2]), third(panel[3]));
    let art = match label {
        Some(l) if !l.text_marker => {
            let mut img = l.img.clone();
            let bx = round_half_even(f64::from(img.w) / (f64::from(panel[2]) / 3.0)).max(1) as u32;
            let by = round_half_even(f64::from(img.h) / (f64::from(panel[3]) / 3.0)).max(1) as u32;
            if img.w > 2 * bx && img.h > 2 * by {
                img = img.crop(bx, by, img.w - bx, img.h - by);
            }
            fit(&img, fw, fh)
        }
        _ => generated_label(title, fw, fh),
    };
    let mut paper = Image::filled(fw, fh, PAPER);
    alpha_composite(&mut paper, &art, 0, 0);
    mask(&mut paper);
    alpha_composite(&mut face, &paper, third(panel[0]), third(panel[1]));
    (face, paper)
}
