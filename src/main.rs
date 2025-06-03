use ferris_the_crawler::{Args, Crawler};

use anyhow::Result;
use clap::Parser;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!("Starting crawler with args: {:?}", args);

    #[cfg(feature = "flamegraph")]
    let guard = pprof::ProfilerGuard::new(100).unwrap();

    let crawler = Crawler::new(args);

    crawler.crawl()?;
    info!(
        "Crawling completed successfully with {} urls",
        crawler.urls().len()
    );

    if let Some(output) = &crawler.args().output {
        crawler.exporter().to_file(output)?;
        info!("Data exported to file: {}", output);
    }

    #[cfg(feature = "flamegraph")]
    {
        if let Ok(report) = guard.report().build() {
            let file = std::fs::File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();
        }
    }

    Ok(())
}
