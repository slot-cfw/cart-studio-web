use std::path::Path;

use slot_cart_studio::advance::plastic::{shell_face, tinted};
use slot_cart_studio::advance::shell::{automatic, layered, parse, Finish, Outline, Shape, Shell};
use slot_cart_studio::advance::{Image, SPECS};

fn dump(name: &str) -> Image {
    let path = Path::new("tests/fixtures/advance/expected/pillow").join(format!("{name}.png"));
    let mut decoder = png::Decoder::new(std::fs::File::open(path).unwrap());
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    Image {
        w: info.width,
        h: info.height,
        px: buf,
    }
}

fn shell(outline: Outline, colour: u32, finish: Finish) -> Shell {
    Shell {
        outline,
        colour: [(colour >> 16) as u8, (colour >> 8) as u8, colour as u8],
        finish,
    }
}

fn gba(code: &str) -> Vec<u8> {
    let mut head = vec![0u8; 0x150];
    head[0xac..0xb0].copy_from_slice(code.as_bytes());
    head
}

fn gb(code: &str, flag: u8, title: &[u8], licensee: u8) -> Vec<u8> {
    let mut head = vec![0u8; 0x150];
    head[0x134..0x134 + title.len()].copy_from_slice(title);
    head[0x13f..0x143].copy_from_slice(format!("{code:<4}").as_bytes());
    head[0x143] = flag;
    head[0x14a] = licensee;
    head
}

#[test]
fn parse_rejects_malformed() {
    for value in [
        "auto zzzzzz solid",
        "rounded ff0000 chrome",
        "auto ff0000 solid extra",
        "nonsense",
    ] {
        assert_eq!(parse(value), None, "{value}");
    }
    assert_eq!(
        parse("auto 8844CC clear"),
        Some(shell(Outline::Auto, 0x8844cc, Finish::Clear))
    );
}

#[test]
fn layered_overrides_and_auto_clears() {
    let out = layered(
        "A = auto ff0000 solid\nB = rounded 223344 clear\n",
        "A = auto 00ff00 glitter\nB = auto\n",
    );
    assert_eq!(
        out.get("A"),
        Some(&shell(Outline::Auto, 0x00ff00, Finish::Glitter))
    );
    assert!(!out.contains_key("B"));
}

#[test]
fn layered_nfc_and_bom() {
    let out = layered("", "\u{feff}Cafe\u{301} = auto 223344 solid\n");
    assert_eq!(
        out.get("Café"),
        Some(&shell(Outline::Auto, 0x223344, Finish::Solid))
    );
}

#[test]
fn automatic_table() {
    assert_eq!(
        automatic("GBA", Some(&gba("AXVE"))),
        shell(Outline::Auto, 0xc2332e, Finish::Clear)
    );
    assert_eq!(
        automatic("GBA", Some(&gba("RZWJ"))),
        shell(Outline::Auto, 0xeceee8, Finish::Solid)
    );
    assert_eq!(
        automatic("GBA", Some(&gba("RZWE"))),
        shell(Outline::Auto, 0x5f6264, Finish::Clear)
    );
    assert_eq!(
        automatic("GBA", Some(&gba("MABC"))),
        shell(Outline::Auto, 0xc6c6c9, Finish::Solid)
    );
    assert_eq!(
        automatic("GB", Some(&gb("", 0xc0, b"", 0))),
        shell(Outline::Rounded, 0x7c7a8a, Finish::Clear)
    );
    assert_eq!(
        automatic("GB", Some(&gb("", 0x80, b"", 0))),
        shell(Outline::Notched, 0x333031, Finish::Solid)
    );
    assert_eq!(
        automatic("GB", Some(&gb("", 0, b"POKEMON RED", 1))),
        shell(Outline::Notched, 0xc0282c, Finish::Solid)
    );
    assert_eq!(
        automatic("GB", Some(&gb("", 0, b"POKEMON RED", 0))),
        shell(Outline::Notched, 0x9a978f, Finish::Solid)
    );
    assert_eq!(
        automatic("GB", Some(&gb("BYTE", 0, b"", 0))),
        shell(Outline::Notched, 0x86b9bf, Finish::Glitter)
    );
    assert_eq!(
        automatic("GBC", None),
        shell(Outline::Auto, 0x7c7a8a, Finish::Clear)
    );
    assert_eq!(
        automatic("GBA", None),
        shell(Outline::Auto, 0x35353a, Finish::Solid)
    );
}

#[test]
fn shapes() {
    use slot_cart_studio::advance::shell::shape_for;
    assert_eq!(
        shape_for("GB", &shell(Outline::Rounded, 0, Finish::Solid)),
        Shape::Gbc
    );
    assert_eq!(
        shape_for("GBC", &shell(Outline::Notched, 0, Finish::Solid)),
        Shape::Gb
    );
    assert_eq!(
        shape_for("GBA", &shell(Outline::Rounded, 0, Finish::Solid)),
        Shape::Gba
    );
    assert_eq!(
        shape_for("GBC", &shell(Outline::Auto, 0, Finish::Solid)),
        Shape::Gbc
    );
}

const COMBOS: [(u32, Finish, &str); 3] = [
    (0x35353a, Finish::Solid, "35353a-solid"),
    (0xc2332e, Finish::Clear, "c2332e-clear"),
    (0x86b9bf, Finish::Glitter, "86b9bf-glitter"),
];

#[test]
fn tinted_matches_pillow() {
    for (shape, name) in [(Shape::Gba, "GBA"), (Shape::Gb, "GB"), (Shape::Gbc, "GBC")] {
        for (colour, finish, tag) in COMBOS {
            let clear = finish != Finish::Solid;
            let file = format!(
                "{}_{}.png",
                name.to_lowercase(),
                if clear && shape != Shape::Gb {
                    "clear"
                } else {
                    "neutral"
                }
            );
            let s = shell(Outline::Auto, colour, finish);
            let out = tinted(&file, s.colour, clear, finish == Finish::Glitter);
            assert!(
                out == dump(&format!("tinted-{name}-{tag}")),
                "tinted {name} {tag}"
            );
        }
    }
}

#[test]
fn shell_face_matches_pillow() {
    for (shape, name) in [(Shape::Gba, "GBA"), (Shape::Gb, "GB"), (Shape::Gbc, "GBC")] {
        for (colour, finish, tag) in COMBOS {
            let s = shell(Outline::Auto, colour, finish);
            let out = shell_face(shape, s.colour, finish, SPECS[shape as usize].1);
            assert!(
                out == dump(&format!("shell_face-{name}-{tag}")),
                "shell_face {name} {tag}"
            );
        }
    }
}

#[test]
fn caches_stay_bounded() {
    use slot_cart_studio::advance::plastic::cache_sizes;
    for n in 0..300u32 {
        let colour = [(n >> 8) as u8, n as u8, 7];
        tinted("gba_neutral.png", colour, false, false);
        shell_face(Shape::Gba, colour, Finish::Solid, SPECS[0].1);
    }
    let (tints, faces) = cache_sizes();
    assert!(tints <= 128 && faces <= 128, "{tints} {faces}");
}
