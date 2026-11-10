use std::collections::{BTreeSet, HashMap, HashSet};

use serde::Serialize;
use unicode_normalization::UnicodeNormalization;

use super::bmp::bmp555;
use super::compose::compose;
use super::decode;
use super::shell::{automatic, layered, parse, Outline, Shell};
use super::{Image, SPECS};

pub struct Converted {
    pub cart: Vec<u8>,
    pub flat: Vec<u8>,
    pub label: Vec<u8>,
}

#[derive(Serialize)]
struct ShellOut {
    outline: &'static str,
    colour: String,
    finish: &'static str,
}

#[derive(Serialize)]
struct Item {
    platform: String,
    stem: String,
    cart: String,
    label: String,
    cart_size: [u32; 2],
    label_size: [u32; 2],
    flat_cart: String,
    shell: ShellOut,
    scraped_label: bool,
    native_text: bool,
    recovered_existing_cart: bool,
    header_read: bool,
}

#[derive(Serialize)]
struct Manifest<'a> {
    format: u32,
    cart_scale: &'static str,
    bmp: &'static str,
    source: &'a str,
    items: Vec<Item>,
    warnings: Vec<String>,
}

pub struct Conversion {
    source: String,
    shells: HashMap<String, Shell>,
    items: Vec<((String, String), Item)>,
    seen: HashSet<(String, String)>,
    roms: Vec<(String, String)>,
}

const PLATFORMS: [&str; 3] = ["GBA", "GB", "GBC"];

fn nfc(s: &str) -> String {
    s.nfc().collect()
}

fn key(s: &str) -> String {
    caseless::default_case_fold_str(&nfc(s))
}

pub fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn cp936(c: char) -> bool {
    let c = c as u32;
    super::cp936::RANGES
        .binary_search_by(|&(lo, hi)| {
            if hi < c {
                std::cmp::Ordering::Less
            } else if lo > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

pub fn validate_name(stem: &str, suffix: &str) -> Result<(), String> {
    if stem.is_empty()
        || stem == "."
        || stem == ".."
        || stem.contains(['/', '\\', '<', '>', ':', '"', '|', '?', '*'])
        || stem.ends_with([' ', '.'])
    {
        return Err(format!("Filename cannot be used on FAT: {}", py_repr(stem)));
    }
    let name = format!("{stem}{suffix}");
    if !name.chars().all(cp936) {
        return Err(format!(
            "Filename is outside the kernel CP936 character set: {}",
            py_repr(stem)
        ));
    }
    let bytes: usize = name.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum();
    if bytes >= 100 {
        return Err(format!(
            "Filename exceeds DS Style's 99-byte record: {name}"
        ));
    }
    Ok(())
}

fn head_of(head: &[u8]) -> Option<&[u8]> {
    (!head.is_empty()).then_some(head)
}

fn settle(platform: &str, head: &[u8], chosen: Option<Shell>) -> Shell {
    let auto = automatic(platform, head_of(head));
    let shell = chosen.unwrap_or(auto);
    if shell.outline == Outline::Auto {
        Shell {
            outline: auto.outline,
            ..shell
        }
    } else {
        shell
    }
}

pub fn effective_shell(platform: &str, head: &[u8], shell: &str) -> Shell {
    settle(platform, head, parse(shell))
}

pub fn cart_size(platform: &str, shell: &Shell) -> (u32, u32) {
    SPECS[super::shell::shape_for(platform, shell) as usize].2
}

pub fn preview(
    platform: &str,
    stem: &str,
    label: &[u8],
    head: &[u8],
    shell: &str,
    flat: bool,
) -> Image {
    let shell = effective_shell(platform, head, shell);
    compose(
        platform,
        &nfc(stem),
        decode::label(label).as_ref(),
        &shell,
        !flat,
    )
    .0
}

impl Conversion {
    pub fn new(source: &str, system_shells: &str, label_shells: &str) -> Conversion {
        Conversion {
            source: source.to_string(),
            shells: layered(system_shells, label_shells),
            items: Vec::new(),
            seen: HashSet::new(),
            roms: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        platform: &str,
        stem: &str,
        label: &[u8],
        head: &[u8],
        rom_suffix: &str,
    ) -> Result<Converted, String> {
        let stem = nfc(stem);
        let default_suffix = if platform == "GBA" {
            ".gba".to_string()
        } else {
            format!(".{}", platform.to_lowercase())
        };
        validate_name(
            &stem,
            if rom_suffix.is_empty() {
                &default_suffix
            } else {
                rom_suffix
            },
        )?;
        let id = (platform.to_string(), key(&stem));
        if self.seen.contains(&id) {
            return Err(format!(
                "Two carts share the artwork stem {}",
                py_repr(&stem)
            ));
        }
        let shell = settle(platform, head, self.shells.get(&stem).copied());
        let decoded = decode::label(label);
        let (cart, small) = compose(platform, &stem, decoded.as_ref(), &shell, true);
        let (flat, _) = compose(platform, &stem, decoded.as_ref(), &shell, false);
        let path = |dir: &str| format!("SYSTEM/SLOT/{dir}/{platform}/{stem}.bmp");
        let item = Item {
            platform: platform.to_string(),
            stem: stem.clone(),
            cart: path("CARTS"),
            label: path("LABELS"),
            cart_size: [cart.w, cart.h],
            label_size: [small.w, small.h],
            flat_cart: path("CARTS_FLAT"),
            shell: ShellOut {
                outline: shell.outline.word(),
                colour: shell.hex(),
                finish: shell.finish.word(),
            },
            scraped_label: !label.is_empty(),
            native_text: label.is_empty() || decoded.as_ref().is_some_and(|l| l.text_marker),
            recovered_existing_cart: false,
            header_read: !rom_suffix.is_empty(),
        };
        if !rom_suffix.is_empty() {
            self.roms.push(id.clone());
        }
        self.seen.insert(id.clone());
        self.items.push((id, item));
        Ok(Converted {
            cart: bmp555(&cart),
            flat: bmp555(&flat),
            label: bmp555(&small),
        })
    }

    pub fn manifest(mut self) -> String {
        self.items.sort_by(|a, b| a.0.cmp(&b.0));
        let stems = |p: &str| -> BTreeSet<&String> {
            self.roms
                .iter()
                .filter(|r| r.0 == p)
                .map(|r| &r.1)
                .collect()
        };
        let mut warnings: Vec<String> = stems("GB")
            .intersection(&stems("GBC"))
            .map(|s| {
                format!(
                    "GB and GBC share the save stem {}. DS Style stores both in SYSTEM/SAVER as .esv; use distinct ROM names to keep saves separate.",
                    py_repr(s)
                )
            })
            .collect();
        for p in PLATFORMS {
            let n = self.items.iter().filter(|(id, _)| id.0 == p).count();
            if n > 512 {
                warnings.push(format!(
                    "{p}: {n} images converted; the kernel displays at most 512 ROMs per shelf."
                ));
            }
        }
        let manifest = Manifest {
            format: 3,
            cart_scale: "1/3",
            bmp: "16-bit BI_RGB RGB555; magenta colour key",
            source: &self.source,
            items: self.items.into_iter().map(|(_, item)| item).collect(),
            warnings,
        };
        let mut out = Vec::new();
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"  ");
        let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
        manifest.serialize(&mut ser).expect("manifest serialises");
        String::from_utf8(out).expect("manifest is UTF-8")
    }
}
