use crate::Url;

use anyhow::{Context, Result};
use tracing::info;

pub struct Exporter {
    pub urls: Vec<Url>,
}

impl Exporter {
    pub fn new(urls: Vec<Url>) -> Self {
        Exporter { urls }
    }

    pub fn csv(&self) -> String {
        let mut csv_data = String::new();

        csv_data.push_str("Target,Source,Depth\n");
        for url in &self.urls {
            csv_data.push_str(&url.to_csv_row());
            csv_data.push('\n');
        }

        csv_data
    }

    pub fn to_file(&self, file_path: &str) -> Result<()> {
        let csv_data = self.csv();
        std::fs::write(file_path, csv_data)
            .context(format!("Failed to write CSV data to file: {}", file_path))?;
        info!("CSV data written to file: {}", file_path);

        Ok(())
    }

    pub fn to_stdout(&self) {
        let csv_data = self.csv();
        println!("{}", csv_data);
    }
}
