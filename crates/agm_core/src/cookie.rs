//! Parse and normalize `cookie.txt` (legacy single-line `key=value; ...` format).

use std::collections::HashMap;

const SKIP_COOKIE: &str = "ckBH_lastBoard";

/// Parses one line of cookie header text into a map.
/// Values containing CJK codepoints are percent-encoded like the Python implementation.
pub fn parse_cookie_line(line: &str) -> HashMap<String, String> {
    let line = line.trim();
    if line.is_empty() {
        return HashMap::new();
    }

    let mut map = HashMap::new();
    for part in line.split("; ") {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        if k == SKIP_COOKIE {
            continue;
        }
        let v = percent_encode_value_if_needed(v);
        map.insert(k.to_string(), v);
    }
    map
}

fn percent_encode_value_if_needed(value: &str) -> String {
    if value.chars().any(is_cjk) {
        percent_encode(value)
    } else {
        value.to_string()
    }
}

fn is_cjk(c: char) -> bool {
    matches!(c, '\u{4e00}'..='\u{9fff}')
}

/// Percent-encode UTF-8 bytes (uppercase hex), for cookie values with non-ASCII.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*b as char);
            }
            _ => {
                out.push('%');
                out.push(hex_upper(b >> 4));
                out.push(hex_upper(b & 0xf));
            }
        }
    }
    out
}

fn hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'A' + (nibble - 10)) as char,
        _ => '0',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_pairs() {
        let m = parse_cookie_line("a=1; b=two");
        assert_eq!(m.get("a").map(String::as_str), Some("1"));
        assert_eq!(m.get("b").map(String::as_str), Some("two"));
    }

    #[test]
    fn skips_ckbh_board() {
        let m = parse_cookie_line("x=1; ckBH_lastBoard=noise; y=2");
        assert!(!m.contains_key("ckBH_lastBoard"));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn encodes_cjk_in_value() {
        let m = parse_cookie_line("k=中文");
        assert_eq!(m.get("k").map(String::as_str), Some("%E4%B8%AD%E6%96%87"));
    }
}
