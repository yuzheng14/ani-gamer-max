/// Raw cookie string as stored in `cookie.txt`.
/// Format: `key=value; key=value; ...` — passed directly as the `Cookie` request header.
#[derive(Debug, Clone, Default)]
pub struct Cookies(pub String);

impl Cookies {
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Cookies {
    fn from(s: String) -> Self {
        Cookies(s)
    }
}
