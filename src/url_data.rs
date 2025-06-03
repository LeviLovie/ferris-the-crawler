#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Url {
    pub url: String,
    pub found_at: String,
    pub depth: usize,
}

impl Url {
    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{}",
            Self::escape_url(&self.url),
            Self::escape_url(&self.found_at),
            self.depth
        )
    }

    fn escape_url(url: &str) -> String {
        url.replace(',', "%2C").replace('\n', "%0A")
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (found at: {}, depth: {})",
            self.url, self.found_at, self.depth
        )
    }
}
