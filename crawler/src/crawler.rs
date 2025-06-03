use crate::{Url, Exporter};

use anyhow::{Context, Result};
use dashmap::DashMap;
use scraper::{Html, Selector};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::info;
use std::{hash::{Hash, Hasher}, collections::hash_map::DefaultHasher};

#[derive(Clone)]
pub struct CrawlerData {
    pub base_url: String,
    pub max_depth: usize,
    pub visited_urls: DashMap<u64, Url>,
}

#[derive(Clone)]
pub struct Crawler {
    pub data: Arc<CrawlerData>,
}

impl Crawler {
    pub fn new(base_url: String, max_depth: usize) -> Self {
        Crawler {
            data: Arc::new(CrawlerData {
                base_url,
                max_depth,
                visited_urls: DashMap::new(),
            }),
        }
    }

    pub fn exporter(&self) -> Exporter {
        Exporter::new(self.urls())
    }

    pub fn add_visited_url(&self, url: Url) {
        self.data.visited_urls.insert(hash_string(&url.url), url);
    }

    pub fn is_visited(&self, url: &str) -> bool {
        self.data.visited_urls.contains_key(&hash_string(url))
    }

    pub fn base_url(&self) -> String {
        self.data.base_url.clone()
    }

    pub fn max_depth(&self) -> usize {
        self.data.max_depth
    }

    pub fn urls(&self) -> Vec<Url> {
        self.data
            .visited_urls
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn crawl(&self) -> Result<()> {
        let rt = Runtime::new().context("Failed to create Tokio runtime")?;

        rt.block_on(self.crawl_url(self.base_url(), "/".to_string(), 0))
    }

    async fn crawl_url(&self, url: String, from: String, depth: usize) -> Result<()> {
        // if !url.contains("https://github.com") {
        //     return Ok(());
        // }

        // let url = url.split("?").next().unwrap_or(&url).to_string();

        if depth > self.max_depth() || self.is_visited(&url) {
            return Ok(());
        }

        self.add_visited_url(Url {
            url: url.clone(),
            found_at: from,
            depth,
        });

        info!("Crawling URL: {}, Depth: {}", url, depth);
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Crawler/1.0)")
            .send()
            .await
            .context(format!("Failed to send request to {}", &url))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Request to {} failed with status: {}",
                url,
                response.status()
            ));
        }

        if depth + 1 > self.max_depth() {
            return Ok(());
        }

        let html = response
            .text()
            .await
            .context("Failed to read response text")?;
        let links = self.extract_links(&html);
        let valid_links: Vec<String> = links
            .into_iter()
            .filter_map(|link| {
                if let Ok(parsed_url) = url::Url::parse(&link) {
                    if parsed_url.host_str() == Some(parsed_url.host_str().unwrap_or("")) {
                        Some(parsed_url.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        info!(
            "Found {} valid links on {} at depth {}",
            valid_links.len(),
            url,
            depth
        );

        let mut tasks = Vec::new();
        for link in valid_links {
            let task = self.spawn_crawl(link, url.clone(), depth + 1);
            tasks.push(task);
        }

        for task in tasks {
            if let Err(e) = task.await {
                return Err(anyhow::anyhow!("Task failed: {:?}", e));
            }
        }

        Ok(())
    }

    fn spawn_crawl(&self, url: String, from: String, depth: usize) -> tokio::task::JoinHandle<Result<()>> {
        let crawler = self.clone();
        tokio::spawn(async move { crawler.crawl_url(url, from, depth).await })
    }

    fn extract_links(&self, html: &str) -> Vec<String> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("a").unwrap();

        document
            .select(&selector)
            .filter_map(|element| element.value().attr("href").map(String::from))
            .collect()
    }
}

fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);

    hasher.finish()
}

