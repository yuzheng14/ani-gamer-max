use std::collections::HashMap;

/// Raw cookie string as stored in `cookie.txt`.
/// Format: `key=value; key=value; ...`
#[derive(Debug, Clone, Default)]
pub struct Cookies(pub String);

impl Cookies {
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses the raw cookie string into a map, matching the original `read_cookie()` behaviour:
    /// - Splits on `"; "` then `"="` (first `=` only)
    /// - Percent-encodes CJK characters (`\u4e00`–`\u9fa5`) in both key and value
    /// - Drops the `ckBH_lastBoard` key
    pub fn to_map(&self) -> HashMap<String, String> {
        self.0
            .split("; ")
            .filter_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                Some((percent_encode_cjk(k), percent_encode_cjk(v)))
            })
            .filter(|(k, _)| k != "ckBH_lastBoard")
            .collect()
    }
}

impl From<String> for Cookies {
    fn from(s: String) -> Self {
        Cookies(s)
    }
}

/// Percent-encodes characters in the CJK Unified Ideographs block (`\u4e00`–`\u9fa5`),
/// matching Python's `urllib.parse.quote(x, safe='')` applied to those characters.
fn percent_encode_cjk(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if ('\u{4e00}'..='\u{9fa5}').contains(&c) {
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            for byte in encoded.as_bytes() {
                out.push('%');
                out.push(char::from_digit((byte >> 4) as u32, 16).unwrap().to_ascii_uppercase());
                out.push(char::from_digit((byte & 0xf) as u32, 16).unwrap().to_ascii_uppercase());
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_map_parses_ascii_cookies() {
        let c = Cookies::from("BAHAID=abc123; BAHARUNE=xyz".to_string());
        let map = c.to_map();
        assert_eq!(map.get("BAHAID").map(|s| s.as_str()), Some("abc123"));
        assert_eq!(map.get("BAHARUNE").map(|s| s.as_str()), Some("xyz"));
    }

    #[test]
    fn to_map_drops_ckbh_lastboard() {
        let c = Cookies::from("BAHAID=abc; ckBH_lastBoard=123; BAHARUNE=xyz".to_string());
        let map = c.to_map();
        assert!(!map.contains_key("ckBH_lastBoard"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn to_map_percent_encodes_cjk_in_value() {
        // '中' is U+4E2D, UTF-8: 0xE4 0xB8 0xAD → %E4%B8%AD
        let c = Cookies::from("key=中".to_string());
        let map = c.to_map();
        assert_eq!(map.get("key").map(|s| s.as_str()), Some("%E4%B8%AD"));
    }

    #[test]
    fn to_map_splits_on_first_equals_only() {
        // value itself contains '='
        let c = Cookies::from("token=abc=def".to_string());
        let map = c.to_map();
        assert_eq!(map.get("token").map(|s| s.as_str()), Some("abc=def"));
    }

    #[test]
    fn to_map_on_empty_cookies_returns_empty_map() {
        let c = Cookies::default();
        assert!(c.to_map().is_empty());
    }

    #[test]
    fn percent_encode_cjk_leaves_ascii_unchanged() {
        assert_eq!(percent_encode_cjk("hello123"), "hello123");
    }

    #[test]
    fn percent_encode_cjk_encodes_cjk_char() {
        // '文' U+6587, UTF-8: 0xE6 0x96 0x87 → %E6%96%87
        assert_eq!(percent_encode_cjk("文"), "%E6%96%87");
    }

    /// Real-world Bahamut cookie shape (desensitized).
    ///
    /// Replaced: BAHAID/BAHANICK/MB_BAHAID/MB_BAHANICK → "testuser";
    /// BAHAHASHID/BAHAENUR/ANIME_SIGN/nologinuser/ckBahamutCsrfToken → zero-filled hex;
    /// BAHARUNE/MB_BAHARUNE → fake JWT with "testuser" payload;
    /// ad-tracking IDs (__gads/__gpi/__eoi) → zeroed; timestamps → 1700000000.
    #[test]
    fn to_map_real_world_bahamut_cookie() {
        let raw = concat!(
            "ANIME_dark_theme=0; ",
            "_ga=GA1.1.1000000000.1700000000; ",
            "buap_puoo=p101; ",
            "nologinuser=0000000000000000000000000000000000000000000000000000000; ",
            "ANIME_SIGN=000000000000000000000000000000000000000000000000000000; ",
            "__gads=ID=0000000000000000:T=1700000000:RT=1700000000:S=ALNI_MafhkTtCwIuH2o5A8I8WFNBdzhD2Q; ",
            "__gpi=UID=0000000000000000:T=1700000000:RT=1700000000:S=ALNI_Mag5-kpL4CXMzqGyClVjYPVTTMHsA; ",
            "__eoi=ID=0000000000000000:T=1700000000:RT=1700000000:S=AA-AfjYDNcxWV4wYq4NNRoZkKrqR; ",
            "ckBahaAd=-------08----------------; ",
            "FCCDCF=%5Bnull%2Cnull%2Cnull%2Cnull%2Cnull%2Cnull%2C%5B%5B32%2C%22%5B%5C%22f6e2a647-2e65-4498-a7d2-11178834d737%5C%22%2C%5B1700000000%2C864000000%5D%5D%22%5D%5D%5D; ",
            "FCNEC=%5B%5B%22AKsRol_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA%3D%3D%22%5D%5D; ",
            "ANIME_NOTICE_PERSONAL_MENU=3; ",
            "ckM=1000000000; ",
            "BAHAID=testuser; ",
            "BAHAHASHID=aabbccddeeff00112233445566778899aabbccddeeff001122334455667788; ",
            "BAHANICK=testuser; ",
            "BAHAENUR=00112233445566778899aabbccddeeff; ",
            "BAHARUNE=eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiJ9.eyJ1c2VyaWQiOiJ0ZXN0dXNlciIsInVzZXJuYW1lIjoidGVzdHVzZXIiLCJtb2JpbGVWZXJpZnkiOmZhbHNlLCJkZW55UG9zdCI6ZmFsc2UsImF2YXRhckxldmVsIjo0LCJtaWQiOjEwMDAwMDAwMDAsIm5vbmNlIjoxMjM0NTY3ODk4LCJqaWQiOiJ0ZXN0dXNlckBsaXRlLmdhbWVyLmNvbS50dyIsImV4cCI6MTc3NzQ5NjQwMH0.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA; ",
            "BAHALV=4; ",
            "BAHAFLT=1600000000; ",
            "MB_BAHAID=testuser; ",
            "MB_BAHANICK=testuser; ",
            "MB_BAHARUNE=eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiJ9.eyJ1c2VyaWQiOiJ0ZXN0dXNlciIsInVzZXJuYW1lIjoidGVzdHVzZXIiLCJtb2JpbGVWZXJpZnkiOmZhbHNlLCJkZW55UG9zdCI6ZmFsc2UsImF2YXRhckxldmVsIjo0LCJtaWQiOjEwMDAwMDAwMDAsIm5vbmNlIjoxMjM0NTY3ODk4LCJqaWQiOiJ0ZXN0dXNlckBsaXRlLmdhbWVyLmNvbS50dyIsImV4cCI6MTc3NzQ5NjQwMH0.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA; ",
            "avtrv=1700000000000; ",
            "age_limit_content=0; ",
            "ga_class1=E; ",
            "_ga_2Q21791Y9D=GS2.1.s1700000000$o1$g0$t1700000000$j53$l0$h0; ",
            "ckBahamutCsrfToken=0000000000000000; ",
            "_ga_MT7EZECMKQ=GS2.1.s1700000000$o1$g1$t1700000000$j17$l0$h0; ",
            "buap_modr=p001",
        );

        let map = Cookies::from(raw.to_string()).to_map();

        // 30 pairs in the string, none are ckBH_lastBoard → 30 entries
        assert_eq!(map.len(), 30);

        assert_eq!(map.get("BAHAID").map(|s| s.as_str()), Some("testuser"));
        assert_eq!(map.get("BAHANICK").map(|s| s.as_str()), Some("testuser"));

        // __gads value itself contains '=' characters — split_once must not truncate it
        assert!(map
            .get("__gads")
            .map(|v| v.starts_with("ID="))
            .unwrap_or(false));

        // BAHARUNE is a JWT (three base64url segments joined by '.') — value must be kept intact
        let rune = map.get("BAHARUNE").expect("BAHARUNE missing");
        assert_eq!(rune.matches('.').count(), 2);

        assert!(!map.contains_key("ckBH_lastBoard"));
    }
}
