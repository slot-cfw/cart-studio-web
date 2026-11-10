use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outline {
    Auto,
    Notched,
    Rounded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Finish {
    Solid,
    Clear,
    Glitter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    Gba = 0,
    Gb = 1,
    Gbc = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shell {
    pub outline: Outline,
    pub colour: [u8; 3],
    pub finish: Finish,
}

impl Outline {
    pub fn word(self) -> &'static str {
        match self {
            Outline::Auto => "auto",
            Outline::Notched => "notched",
            Outline::Rounded => "rounded",
        }
    }
}

impl Finish {
    pub fn word(self) -> &'static str {
        match self {
            Finish::Solid => "solid",
            Finish::Clear => "clear",
            Finish::Glitter => "glitter",
        }
    }
}

impl Shell {
    pub fn hex(&self) -> String {
        self.colour.iter().map(|c| format!("{c:02x}")).collect()
    }
}

const fn rgb(v: u32) -> [u8; 3] {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

const S: Finish = Finish::Solid;
const C: Finish = Finish::Clear;
const G: Finish = Finish::Glitter;

const GBA_CODES: &[(&str, u32, Finish)] = &[
    ("AXV", 0xc2332e, C),
    ("AXP", 0x2f5cc0, C),
    ("BPE", 0x399c3c, C),
    ("BPR", 0xd85224, S),
    ("BPG", 0x63b044, S),
    ("U3I", 0xd9dbd8, C),
    ("U32", 0xd9dbd8, C),
    ("U33", 0xd9dbd8, C),
    ("V49", 0xa9513b, S),
    ("KYGE", 0x2f8f4e, S),
    ("KYGP", 0x2f8f4e, S),
    ("RZW", 0x5f6264, C),
    ("RZWJ", 0xeceee8, S),
];

const GB_CODES: &[(&str, u32, Finish)] = &[
    ("AAU", 0xb38b3a, S),
    ("AAUJ", 0x333031, S),
    ("AAX", 0xa9aaa7, S),
    ("AAXJ", 0x333031, S),
    ("BYT", 0x86b9bf, G),
    ("BXT", 0x86b9bf, G),
    ("KTN", 0xec94b4, C),
    ("KKK", 0xec94b4, C),
    ("KCE", 0x1f9fb6, C),
    ("VCA", 0xf0a95e, C),
    ("VPHJ", 0xe2b413, S),
    ("BMG", 0x333031, S),
];

fn lookup(code: &str, table: &[(&str, u32, Finish)]) -> Option<([u8; 3], Finish)> {
    let find = |k: &str| {
        table
            .iter()
            .find(|(c, _, _)| *c == k)
            .map(|(_, v, f)| (rgb(*v), *f))
    };
    find(code).or_else(|| {
        let three: String = code.chars().take(3).collect();
        find(&three)
    })
}

fn ascii_replace(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| if b < 0x80 { b as char } else { '\u{fffd}' })
        .collect()
}

pub(crate) fn py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

pub(crate) fn py_strip(s: &str) -> &str {
    s.trim_matches(py_space)
}

fn py_lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let breaks = matches!(
            c,
            '\n' | '\r'
                | '\u{0b}'
                | '\u{0c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if breaks {
            out.push(&text[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r' {
                if let Some(&(j, '\n')) = chars.peek() {
                    chars.next();
                    end = j + 1;
                }
            }
            start = end;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

fn parse_ini(text: &str) -> Vec<(String, String)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out = Vec::new();
    for line in py_lines(text) {
        let line = py_strip(line);
        if line.is_empty() || line.starts_with(['#', ';', '[']) || !line.contains('=') {
            continue;
        }
        let (name, value) = line.split_once('=').unwrap();
        out.push((py_strip(name).nfc().collect(), py_strip(value).to_string()));
    }
    out
}

pub fn parse(value: &str) -> Option<Shell> {
    let words: Vec<&str> = value.split(py_space).filter(|w| !w.is_empty()).collect();
    let [outline, colour, finish] = words[..] else {
        return None;
    };
    let outline = match outline {
        "auto" => Outline::Auto,
        "notched" => Outline::Notched,
        "rounded" => Outline::Rounded,
        _ => return None,
    };
    if colour.len() != 6 || !colour.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let finish = match finish {
        "solid" => Finish::Solid,
        "clear" => Finish::Clear,
        "glitter" => Finish::Glitter,
        _ => return None,
    };
    Some(Shell {
        outline,
        colour: rgb(u32::from_str_radix(colour, 16).ok()?),
        finish,
    })
}

pub fn layered(system: &str, labels: &str) -> HashMap<String, Shell> {
    let mut out: HashMap<String, Shell> = parse_ini(system)
        .into_iter()
        .filter_map(|(k, v)| parse(&v).map(|s| (k, s)))
        .collect();
    for (k, v) in parse_ini(labels) {
        if v == "auto" {
            out.remove(&k);
        } else if let Some(s) = parse(&v) {
            out.insert(k, s);
        }
    }
    out
}

pub fn automatic(platform: &str, head: Option<&[u8]>) -> Shell {
    let (mut colour, mut finish) = match platform {
        "GB" => (rgb(0x9a978f), S),
        "GBC" => (rgb(0x7c7a8a), C),
        _ => (rgb(0x35353a), S),
    };
    let mut outline = Outline::Auto;
    if let Some(head) = head {
        if platform == "GBA" && head.len() >= 0xb0 {
            let code = ascii_replace(&head[0xac..0xb0]);
            if let Some(choice) = lookup(&code, GBA_CODES) {
                (colour, finish) = choice;
            } else if code.starts_with('M') {
                (colour, finish) = (rgb(0xc6c6c9), S);
            }
        } else if (platform == "GB" || platform == "GBC") && head.len() >= 0x14b {
            let flag = head[0x143];
            outline = if flag == 0xc0 {
                Outline::Rounded
            } else {
                Outline::Notched
            };
            (colour, finish) = match flag {
                0xc0 => (rgb(0x7c7a8a), C),
                0x80 => (rgb(0x333031), S),
                _ => (rgb(0x9a978f), S),
            };
            let mut code = ascii_replace(&head[0x13f..0x143]);
            if !(code.len() == 4
                && code
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()))
            {
                code.clear();
            }
            if let Some(choice) = lookup(&code, GB_CODES) {
                (colour, finish) = choice;
            } else if head[0x14a] != 0 {
                let raw = &head[0x134..0x13f];
                let title = ascii_replace(raw.split(|&b| b == 0).next().unwrap_or(raw));
                for (prefix, ink) in [
                    ("POKEMON RED", 0xc0282c),
                    ("POKEMON BLU", 0x2b3a88),
                    ("POKEMON YEL", 0xe9a826),
                ] {
                    if title == prefix {
                        (colour, finish) = (rgb(ink), S);
                    }
                }
            }
        }
    }
    Shell {
        outline,
        colour,
        finish,
    }
}

pub fn shape_for(platform: &str, shell: &Shell) -> Shape {
    let own = match platform {
        "GB" => Shape::Gb,
        "GBC" => Shape::Gbc,
        _ => Shape::Gba,
    };
    if own == Shape::Gba {
        return own;
    }
    match shell.outline {
        Outline::Notched => Shape::Gb,
        Outline::Rounded => Shape::Gbc,
        Outline::Auto => own,
    }
}
