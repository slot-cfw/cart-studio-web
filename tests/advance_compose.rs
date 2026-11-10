mod advance_common;

use std::path::{Path, PathBuf};

use advance_common::{check, dump, FIX};

use slot_cart_studio::advance::bmp::bmp555;
use slot_cart_studio::advance::compose::compose;
use slot_cart_studio::advance::decode;
use slot_cart_studio::advance::shell::parse;
use slot_cart_studio::advance::text::generated_label;
use slot_cart_studio::advance::Image;

fn label_file(platform: &str, stem: &str) -> Option<PathBuf> {
    let dir = Path::new(FIX).join("card/Labels").join(platform);
    ["png", "jpg", "jpeg", "webp", "bmp"]
        .iter()
        .map(|e| dir.join(format!("{stem}.{e}")))
        .find(|p| p.exists())
}

#[test]
fn generated_label_matches_pillow() {
    let titles: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(Path::new(FIX).join("expected/pillow/titles.json")).unwrap(),
    )
    .unwrap();
    for (n, title) in titles.iter().enumerate() {
        for (w, h) in [(88, 45), (74, 66)] {
            assert!(
                generated_label(title, w, h) == dump(&format!("label-{n}-{w}x{h}")),
                "{title} {w}x{h}"
            );
        }
    }
}

#[test]
fn bmp_header_and_key() {
    let mut im = Image::filled(3, 2, [10, 20, 30, 255]);
    im.set(0, 0, [0, 0, 0, 0]);
    im.set(1, 0, [255, 0, 255, 255]);
    let bmp = bmp555(&im);
    assert_eq!(&bmp[..2], b"BM");
    assert_eq!(bmp.len(), 54 + 8 * 2);
    assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()), 70);
    let top = &bmp[54 + 8..];
    assert_eq!(u16::from_le_bytes([top[0], top[1]]), 0x7c1f);
    assert_eq!(u16::from_le_bytes([top[2], top[3]]), 0x7c1e);
}

#[test]
fn cart_bmps_match_exe() {
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(Path::new(FIX).join("expected/SYSTEM/SLOT/conversion.json"))
            .unwrap(),
    )
    .unwrap();
    for item in manifest["items"].as_array().unwrap() {
        let platform = item["platform"].as_str().unwrap();
        let stem = item["stem"].as_str().unwrap();
        let s = &item["shell"];
        let shell = parse(&format!(
            "{} {} {}",
            s["outline"].as_str().unwrap(),
            s["colour"].as_str().unwrap(),
            s["finish"].as_str().unwrap()
        ))
        .unwrap();
        let label =
            label_file(platform, stem).and_then(|p| decode::label(&std::fs::read(p).unwrap()));
        let (cart, small) = compose(platform, stem, label.as_ref(), &shell, true);
        let (flat, _) = compose(platform, stem, label.as_ref(), &shell, false);
        for (key, img) in [("cart", &cart), ("flat_cart", &flat), ("label", &small)] {
            let path = item[key].as_str().unwrap();
            let want = std::fs::read(Path::new(FIX).join("expected").join(path)).unwrap();
            check(path, &bmp555(img), &want);
        }
    }
}

#[test]
fn half_even_crop() {
    let label = decode::label(&std::fs::read(label_file("GBA", "Logo").unwrap()).unwrap()).unwrap();
    assert_eq!((label.img.w, label.img.h), (220, 113));
    let (_, small) = compose(
        "GBA",
        "Logo",
        Some(&label),
        &parse("auto 35353a solid").unwrap(),
        true,
    );
    let want =
        std::fs::read(Path::new(FIX).join("expected/SYSTEM/SLOT/LABELS/GBA/Logo.bmp")).unwrap();
    assert!(bmp555(&small) == want);
}
