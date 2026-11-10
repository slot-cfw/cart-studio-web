use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageReader};

use super::Image;

pub struct Label {
    pub img: Image,
    pub text_marker: bool,
}

const LIMIT: u64 = 64 << 20;

fn other(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_decoder()
        .ok()?;
    if decoder.total_bytes() > LIMIT {
        return None;
    }
    let orientation = decoder.orientation().ok()?;
    let mut img = DynamicImage::from_decoder(decoder).ok()?;
    img.apply_orientation(orientation);
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some((rgba.into_raw(), w, h))
}

pub fn rgba(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    if bytes.starts_with(b"\x89PNG") || bytes.starts_with(b"RIFF") {
        crate::art::decode_png_bytes(bytes)
    } else {
        other(bytes)
    }
}

fn marked(bytes: &[u8]) -> bool {
    if !bytes.starts_with(b"\x89PNG") {
        return false;
    }
    let Ok(reader) = png::Decoder::new(Cursor::new(bytes)).read_info() else {
        return false;
    };
    let info = reader.info();
    let latin = info
        .uncompressed_latin1_text
        .iter()
        .map(|t| (t.keyword.clone(), t.text.clone()));
    let utf8 = info
        .utf8_text
        .iter()
        .filter_map(|t| t.get_text().ok().map(|text| (t.keyword.clone(), text)));
    latin
        .chain(utf8)
        .any(|(k, v)| k == "slot-label" && v.starts_with("text "))
}

pub fn label(bytes: &[u8]) -> Option<Label> {
    if bytes.is_empty() {
        return None;
    }
    let (px, w, h) = rgba(bytes)?;
    Some(Label {
        img: Image { w, h, px },
        text_marker: marked(bytes),
    })
}
