use anyhow::Result;
use crawler::Crawler;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    #[cfg(feature = "flamegraph")]
    let guard = pprof::ProfilerGuard::new(100).unwrap();

    let crawler = Crawler::new("https://github.com/sindresorhus/awesome".to_string(), 1);

    crawler.crawl()?;
    info!(
        "Crawling completed successfully with {} urls",
        crawler.urls().len()
    );

    let exporter = crawler.exporter();
    exporter.to_file("data.csv")?;

    #[cfg(feature = "flamegraph")]
    {
        if let Ok(report) = guard.report().build() {
            let file = std::fs::File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();
        }
    }

    Ok(())
}
