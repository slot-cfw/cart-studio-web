mod advance_common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use advance_common::{check, FIX};
use slot_cart_studio::advance::convert::{preview, validate_name, Conversion};

fn casefold(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    caseless::default_case_fold_str(&s.nfc().collect::<String>())
}

struct Item {
    platform: String,
    stem: String,
    label: Vec<u8>,
    head: Vec<u8>,
    suffix: String,
}

fn walk(card: &Path) -> Vec<Item> {
    let mut items: BTreeMap<(String, String), Item> = BTreeMap::new();
    for (platform, exts) in [
        ("GBA", &["gba", "agb"][..]),
        ("GB", &["gb"][..]),
        ("GBC", &["gbc"][..]),
    ] {
        for entry in fs::read_dir(card.join("Games").join(platform)).unwrap() {
            let path = entry.unwrap().path();
            let ext = path.extension().unwrap().to_str().unwrap().to_lowercase();
            if !exts.contains(&ext.as_str()) {
                continue;
            }
            let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
            let head = fs::read(&path).unwrap()[..0x150].to_vec();
            let suffix = format!(".{}", path.extension().unwrap().to_str().unwrap());
            items.insert(
                (platform.to_string(), casefold(&stem)),
                Item {
                    platform: platform.into(),
                    stem,
                    label: Vec::new(),
                    head,
                    suffix,
                },
            );
        }
        for entry in fs::read_dir(card.join("Labels").join(platform)).unwrap() {
            let path = entry.unwrap().path();
            let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
            let item = items
                .entry((platform.to_string(), casefold(&stem)))
                .or_insert(Item {
                    platform: platform.into(),
                    stem,
                    label: Vec::new(),
                    head: Vec::new(),
                    suffix: String::new(),
                });
            item.label = fs::read(&path).unwrap();
        }
    }
    items.into_values().collect()
}

#[test]
fn fixture_card_matches_exe() {
    let card = Path::new(FIX).join("card");
    let expected = Path::new(FIX).join("expected");
    let system = fs::read_to_string(card.join("System/cart_shell.txt")).unwrap();
    let labels = fs::read_to_string(card.join("Labels/cart_shell.txt")).unwrap();
    let mut conversion = Conversion::new("card", &system, &labels);
    for item in walk(&card) {
        let out = conversion
            .add(
                &item.platform,
                &item.stem,
                &item.label,
                &item.head,
                &item.suffix,
            )
            .unwrap();
        for (dir, bytes) in [
            ("CARTS", &out.cart),
            ("CARTS_FLAT", &out.flat),
            ("LABELS", &out.label),
        ] {
            let path = format!("SYSTEM/SLOT/{dir}/{}/{}.bmp", item.platform, item.stem);
            check(&path, bytes, &fs::read(expected.join(&path)).unwrap());
        }
    }
    let want = fs::read_to_string(expected.join("SYSTEM/SLOT/conversion.json")).unwrap();
    assert_eq!(conversion.manifest(), want);
}

#[test]
fn manifest_warnings() {
    let mut conversion = Conversion::new("card", "", "");
    for n in 0..513 {
        conversion
            .add("GBA", &format!("Game {n}"), &[], &[], "")
            .unwrap();
    }
    conversion
        .add("GB", "Twin's", &[], &[0; 0x150], ".gb")
        .unwrap();
    conversion
        .add("GBC", "Twin's", &[], &[0; 0x150], ".gbc")
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&conversion.manifest()).unwrap();
    assert_eq!(
        manifest["warnings"],
        serde_json::json!([
            "GB and GBC share the save stem \"twin's\". DS Style stores both in SYSTEM/SAVER as .esv; use distinct ROM names to keep saves separate.",
            "GBA: 513 images converted; the kernel displays at most 512 ROMs per shelf."
        ])
    );
}

#[test]
fn duplicate_stems_refused() {
    let mut conversion = Conversion::new("card", "", "");
    conversion.add("GBA", "Game", &[], &[], "").unwrap();
    assert!(conversion.add("GBA", "GAME", &[], &[], "").is_err());
}

#[test]
fn validate_name_rules() {
    assert_eq!(
        validate_name("a/b", ".gba").unwrap_err(),
        "Filename cannot be used on FAT: 'a/b'"
    );
    assert!(validate_name("trailing.", ".gba").is_err());
    assert!(validate_name("", ".gba").is_err());
    assert_eq!(
        validate_name(&"x".repeat(96), ".gba").unwrap_err(),
        format!(
            "Filename exceeds DS Style's 99-byte record: {}.gba",
            "x".repeat(96)
        )
    );
    assert!(validate_name(&"x".repeat(95), ".gba").is_ok());
    let emoji = char::from_u32(0x1F600).unwrap().to_string();
    assert!(validate_name(&emoji, ".gba")
        .unwrap_err()
        .starts_with("Filename is outside the kernel CP936"));
    assert!(validate_name("Brûlée", ".gba").is_err());
    assert!(validate_name("ポケモン", ".gba").is_ok());
}

#[test]
fn preview_auto_outline() {
    let mut head = vec![0u8; 0x150];
    head[0x143] = 0xc0;
    let img = preview("GB", "Pak", &[], &head, "auto 123456 solid", false);
    assert_eq!((img.w, img.h), (104, 112));
    let same = preview("GBC", "Pak", &[], &head, "rounded 123456 solid", false);
    assert!(img == same);
    let gba = preview("GBA", "Cart", &[], &[], "", true);
    assert_eq!((gba.w, gba.h), (120, 71));
}
