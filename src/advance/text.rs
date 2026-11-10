use super::assets::DS_FONT;
use super::shell::{py_space, py_strip};
use super::Image;

const PAPER: [u8; 4] = [0xe9, 0xe4, 0xd8, 255];
const INK: [u8; 4] = [40, 40, 47, 255];

fn ws(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | ' ')
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn letter(c: char) -> bool {
    word(c) && !c.is_numeric()
}

fn punct(c: char) -> bool {
    word(c) || matches!(c, '!' | '"' | '\'' | '&' | '.' | ',' | '?')
}

fn em_dash_at(s: &[char], i: usize) -> bool {
    let mut j = i;
    while j < s.len() && s[j] == '-' {
        j += 1;
    }
    j - i >= 2 && j < s.len() && word(s[j])
}

fn word_end(s: &[char], i: usize) -> usize {
    let mut k = i + 1;
    loop {
        if k < s.len() && s[k] == '-' {
            let after = k + 1;
            let behind = (k >= 2 && letter(s[k - 2]) && letter(s[k - 1]))
                || (k >= 3 && letter(s[k - 3]) && s[k - 2] == '-' && letter(s[k - 1]));
            let ahead = after < s.len()
                && letter(s[after])
                && ((after + 1 < s.len() && letter(s[after + 1]))
                    || (after + 2 < s.len() && s[after + 1] == '-' && letter(s[after + 2])));
            if behind && ahead {
                return after;
            }
        }
        if k == s.len() || ws(s[k]) {
            return k;
        }
        if punct(s[k - 1]) && em_dash_at(s, k) {
            return k;
        }
        k += 1;
    }
}

fn split(text: &str) -> Vec<String> {
    let s: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let end = if ws(s[i]) {
            let mut j = i;
            while j < s.len() && ws(s[j]) {
                j += 1;
            }
            j
        } else if i > 0 && punct(s[i - 1]) && em_dash_at(&s, i) {
            let mut j = i;
            while s[j] == '-' {
                j += 1;
            }
            j
        } else {
            word_end(&s, i)
        };
        chunks.push(s[i..end].iter().collect());
        i = end;
    }
    chunks
}

fn munge(text: &str) -> String {
    let mut out = String::new();
    let mut col = 0;
    for c in text.chars() {
        match c {
            '\t' => {
                let n = 8 - col % 8;
                out.extend(std::iter::repeat_n(' ', n));
                col += n;
            }
            '\n' | '\r' => {
                out.push(' ');
                col = 0;
            }
            c if ws(c) => {
                out.push(' ');
                col += 1;
            }
            c => {
                out.push(c);
                col += 1;
            }
        }
    }
    out
}

fn len(s: &str) -> usize {
    s.chars().count()
}

fn blank(s: &str) -> bool {
    py_strip(s).is_empty()
}

#[allow(clippy::manual_pop_if)]
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut chunks = split(&munge(text));
    chunks.reverse();
    let mut lines: Vec<String> = Vec::new();
    while !chunks.is_empty() {
        let mut cur: Vec<String> = Vec::new();
        let mut cur_len = 0;
        if blank(chunks.last().unwrap()) && !lines.is_empty() {
            chunks.pop();
        }
        while let Some(last) = chunks.last() {
            let l = len(last);
            if cur_len + l <= width {
                cur.push(chunks.pop().unwrap());
                cur_len += l;
            } else {
                break;
            }
        }
        if chunks.last().is_some_and(|c| len(c) > width) {
            let space_left = if width < 1 { 1 } else { width - cur_len };
            if space_left > 0 {
                let chunk: Vec<char> = chunks.last().unwrap().chars().collect();
                let mut end = space_left;
                if chunk.len() > space_left {
                    if let Some(hyphen) = chunk[..space_left].iter().rposition(|&c| c == '-') {
                        if hyphen > 0 && chunk[..hyphen].iter().any(|&c| c != '-') {
                            end = hyphen + 1;
                        }
                    }
                }
                cur.push(chunk[..end].iter().collect());
                *chunks.last_mut().unwrap() = chunk[end..].iter().collect();
            } else if cur.is_empty() {
                cur.push(chunks.pop().unwrap());
            }
        }
        cur.pop_if(|c| blank(c));
        if !cur.is_empty() {
            lines.push(cur.concat());
        }
    }
    lines
}

fn clean(title: &str) -> String {
    let s: Vec<char> = title.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    'scan: while i < s.len() {
        let mut j = i;
        while j < s.len() && py_space(s[j]) {
            j += 1;
        }
        if j < s.len() {
            let close = match s[j] {
                '(' => Some(')'),
                '[' => Some(']'),
                _ => None,
            };
            if let Some(close) = close {
                if let Some(k) = s[j + 1..].iter().position(|&c| c == close) {
                    i = j + 1 + k + 1;
                    continue 'scan;
                }
            }
        }
        out.push(s[i]);
        i += 1;
    }
    let stripped = py_strip(&out);
    if stripped.is_empty() {
        title.to_string()
    } else {
        stripped.to_string()
    }
}

pub fn generated_label(title: &str, w: u32, h: u32) -> Image {
    let mut im = Image::filled(w, h, PAPER);
    let clean = clean(title);
    let width = ((w as i64 - 8).div_euclid(6)).max(1) as usize;
    let most = ((h as i64 - 4).div_euclid(12)).max(1) as usize;
    let lines: Vec<String> = wrap(&clean, width).into_iter().take(most).collect();
    let mut y = (h as i64 - lines.len() as i64 * 12).div_euclid(2);
    for line in &lines {
        let mut x = (w as i64 - len(line) as i64 * 6).div_euclid(2);
        for c in line.chars() {
            let code = if (32..127).contains(&(c as u32)) {
                c as usize
            } else {
                '?' as usize
            };
            for row in 0..12 {
                let bits = DS_FONT[code * 12 + row];
                for col in 0..8 {
                    let (xx, yy) = (x + col, y + row as i64);
                    if bits & (128 >> col) != 0
                        && (0..w as i64).contains(&xx)
                        && (0..h as i64).contains(&yy)
                    {
                        im.set(xx as u32, yy as u32, INK);
                    }
                }
            }
            x += 6;
        }
        y += 12;
    }
    im
}
