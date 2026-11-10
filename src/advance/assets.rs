use super::Image;

pub const DS_FONT: &[u8] = include_bytes!("../../assets/advance/ds-font.bin");

fn bytes(name: &str) -> &'static [u8] {
    match name {
        "board.png" => include_bytes!("../../assets/advance/board.png"),
        "gb_board.png" => include_bytes!("../../assets/advance/gb_board.png"),
        "gb_interior.png" => include_bytes!("../../assets/advance/gb_interior.png"),
        "gb_neutral.png" => include_bytes!("../../assets/advance/gb_neutral.png"),
        "gb_simple_mould.png" => include_bytes!("../../assets/advance/gb_simple_mould.png"),
        "gba_clear.png" => include_bytes!("../../assets/advance/gba_clear.png"),
        "gba_interior.png" => include_bytes!("../../assets/advance/gba_interior.png"),
        "gba_neutral.png" => include_bytes!("../../assets/advance/gba_neutral.png"),
        "gba_simple_mould.png" => include_bytes!("../../assets/advance/gba_simple_mould.png"),
        "gbc_clear.png" => include_bytes!("../../assets/advance/gbc_clear.png"),
        "gbc_interior.png" => include_bytes!("../../assets/advance/gbc_interior.png"),
        "gbc_neutral.png" => include_bytes!("../../assets/advance/gbc_neutral.png"),
        "gbc_simple_mould.png" => include_bytes!("../../assets/advance/gbc_simple_mould.png"),
        _ => panic!("no slot. advance asset {name}"),
    }
}

pub fn png(name: &str) -> Image {
    let (px, w, h) = crate::art::decode(bytes(name)).expect("bundled asset decodes");
    Image { w, h, px }
}
