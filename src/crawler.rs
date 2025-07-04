use crate::{Args, Exporter, GephiClient, Url};

use anyhow::{Context, Result};
use dashmap::DashMap;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use tokio::runtime;
use tokio::sync::Mutex;
use tracing::{error, info};

#[derive(Clone)]
pub struct CrawlerData {
    pub args: Args,
    pub visited_urls: DashMap<u64, Url>,
}

#[derive(Clone)]
pub struct Crawler {
    pub data: Arc<CrawlerData>,
    pub tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<Result<()>>>>>,
    pub send_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<Result<()>>>>>,
    pub gephi_client: Option<Arc<Mutex<GephiClient>>>,
}

impl Crawler {
    pub fn new(args: Args) -> Self {
        let gephi_client = if args.gephi_url.is_empty() {
            None
        } else {
            Some(Arc::new(Mutex::new(GephiClient::new(&args.gephi_url))))
        };

        Crawler {
            data: Arc::new(CrawlerData {
                args,
                visited_urls: DashMap::new(),
            }),
            tasks: Arc::new(Mutex::new(Vec::new())),
            send_tasks: Arc::new(Mutex::new(Vec::new())),
            gephi_client,
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

    pub fn args(&self) -> &Args {
        &self.data.args
    }

    pub fn urls(&self) -> Vec<Url> {
        self.data
            .visited_urls
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub async fn add_task(&self, task: tokio::task::JoinHandle<Result<()>>) {
        self.tasks.lock().await.push(task);
    }

    pub async fn add_send_task(&self, task: tokio::task::JoinHandle<Result<()>>) {
        self.send_tasks.lock().await.push(task);
    }

    pub async fn gephi_add(&self, source: &str, target: &str, depth: usize) -> Result<()> {
        match self.args().command {
            crate::args::Command::Wiki { amount: _, link: _ } => {
                if source.contains("Random") {
                    return Ok(());
                }
            }
            _ => {}
        }

        if let Some(client) = &self.gephi_client {
            info!(
                "Adding edge from {} to {} at depth {}",
                source, target, depth
            );

            let client = client.lock().await;
            match client.add_node(source, source, depth).await {
                Ok(_) => info!("Node {} added successfully", source),
                Err(e) => error!("Failed to add node {}: {:?}", source, e),
            };
            match client.add_node(target, target, depth + 1).await {
                Ok(_) => info!("Node {} added successfully", target),
                Err(e) => error!("Failed to add node {}: {:?}", target, e),
            };
            match client
                .add_edge(&format!("{}-{}", source, target), source, target, true)
                .await
            {
                Ok(_) => info!("Edge {} -> {} added successfully", source, target),
                Err(e) => error!("Failed to add edge {} -> {}: {:?}", source, target, e),
            };
        }

        Ok(())
    }

    pub fn crawl(&self) -> Result<()> {
        let rt = runtime::Builder::new_multi_thread()
            .worker_threads(self.args().threads as usize)
            .enable_all()
            .build()
            .context("Failed to create Tokio runtime")?;

        rt.block_on(async {
            match self.args().command {
                crate::args::Command::Html => {
                    self.add_task(self.spawn_crawl(
                        self.args().url.to_string(),
                        self.args().url.to_string(),
                        0,
                    ))
                    .await;
                }
                crate::args::Command::Wiki { amount, link: _ } => {
                    for _ in 0..amount {
                        self.add_task(self.spawn_crawl(
                            self.args().url.to_string(),
                            self.args().url.to_string(),
                            0,
                        ))
                        .await;
                    }
                }
            }

            self.wait_for_tasks().await?;

            Ok(())
        })
    }

    async fn crawl_html_url(&self, url: String, from: String, depth: usize) -> Result<()> {
        if depth > self.args().depth as usize {
            return Ok(());
        }

        let mut url = url;
        if url.starts_with("/") {
            let base_url = self.args().url.clone();
            let full_url = format!("{}{}", base_url, url);
            url = full_url;
        }
        let url_struct: url::Url = match url.parse() {
            Ok(parsed_url) => parsed_url,
            Err(_) => {
                return Ok(());
            }
        };
        let url = match self.args().ignore_query {
            false => url_struct.to_string(),
            true => {
                let mut url_without_query = url_struct.clone();
                url_without_query.set_query(None);
                url_without_query.to_string()
            }
        };

        if self.args().filters.len() > 0 {
            let mut allowed = false;
            for filter in &self.args().filters {
                if url.contains(filter) {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                info!("Skipping: {} (not in filters)", url);
                return Ok(());
            }
        }
        if self.args().ignore.len() > 0 {
            let mut ignored = false;
            for ignore in &self.args().ignore {
                if url.contains(ignore) {
                    ignored = true;
                    break;
                }
            }
            if ignored {
                info!("Skipping: {} (in ignore list)", url);
                return Ok(());
            }
        }

        self.add_send_task(self.spawn_add_gephi(from.clone(), url.clone(), depth))
            .await;

        if self.is_visited(&url) {
            info!("Already visited: {}", url);
            return Ok(());
        }

        self.add_visited_url(Url {
            url: url.clone(),
            found_at: from.clone(),
            depth,
        });

        if depth + 1 > self.args().depth as usize {
            info!("Max depth: {}", url);
            return Ok(());
        }

        info!("Crawling (depth: {}): {}", depth, url);
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
        info!("Fetched: {}", url);

        let html = response
            .text()
            .await
            .context("Failed to read response text")?;
        let links = self.extract_links(&html);
        info!("Found {} links on {} at depth {}", links.len(), url, depth);

        for link in links {
            self.add_task(self.spawn_crawl(link, url.clone(), depth + 1))
                .await;
        }

        Ok(())
    }

    pub async fn wait_for_tasks(&self) -> Result<()> {
        let self_clone1 = self.clone();
        let self_clone2 = self.clone();

        let send_task_loop = tokio::spawn(async move {
            loop {
                let send_tasks: Vec<_> = {
                    let mut locked = self_clone1.send_tasks.lock().await;
                    std::mem::take(&mut *locked)
                };

                if !send_tasks.is_empty() {
                    let results = futures::future::join_all(send_tasks).await;

                    for result in results {
                        if let Err(e) = result {
                            error!("Send task failed: {:?}", e);
                        }
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });

        let regular_task_loop = tokio::spawn(async move {
            loop {
                let tasks: Vec<_> = {
                    let mut locked = self_clone2.tasks.lock().await;
                    if locked.is_empty() {
                        break;
                    }
                    std::mem::take(&mut *locked)
                };

                let results = futures::future::join_all(tasks).await;

                for result in results {
                    if let Err(e) = result {
                        error!("Task failed: {:?}", e);
                    }
                }
            }
        });

        // Wait for regular tasks to finish; send_task_loop is infinite, so we detach it.
        let (_, regular_result) = futures::join!(send_task_loop, regular_task_loop);
        regular_result.map_err(|e| anyhow::anyhow!("Task join error: {:?}", e))?;

        Ok(())
    }

    async fn crawl_url(&self, url: String, from: String, depth: usize) -> Result<()> {
        match self.args().command {
            crate::args::Command::Html => self.crawl_html_url(url, from, depth).await,
            crate::args::Command::Wiki { amount: _, link: _ } => {
                self.crawl_wiki_url(url, from, depth).await
            }
        }
    }

    async fn crawl_wiki_url(&self, url: String, from: String, depth: usize) -> Result<()> {
        if depth > self.args().depth as usize {
            return Ok(());
        }

        self.add_send_task(self.spawn_add_gephi(from.clone(), url.clone(), depth))
            .await;

        if self.is_visited(&url) {
            info!("Already visited: {}", url);
            return Ok(());
        }

        if !url.contains("Random") {
            tracing::warn!("Crawling non-random URL: {}", url);
            let parts = url.split("/wiki/").collect::<Vec<_>>();
            if parts.len() < 2 {
                info!("Invalid URL format: {}", url);
                return Ok(());
            }
            if parts[1].contains(":") || parts[1].contains("#") {
                info!("Skipping non-article URL: {}", url);
                return Ok(());
            }

            self.add_visited_url(Url {
                url: url.clone(),
                found_at: from.clone(),
                depth,
            });
        }

        if depth + 1 > self.args().depth as usize {
            info!("Max depth: {}", url);
            return Ok(());
        }

        info!("Crawling (depth: {}): {}", depth, url);
        let reqsponse = reqwest::get(&url)
            .await
            .context(format!("Failed to send request to {}", &url))?;
        let html = reqsponse
            .text()
            .await
            .context("Failed to read response text")?;
        info!("Fetched: {}", url);

        let links = {
            let doc = Html::parse_document(&html);
            let selector =
                Selector::parse("#mw-content-text .mw-parser-output p a[href^=\"/wiki/\"]")
                    .unwrap();

            doc.select(&selector)
                .filter_map(|element| {
                    let href = element.value().attr("href")?;
                    if href.contains(":") || href.contains("#") {
                        return None;
                    }
                    let full_url = url::Url::parse("https://en.wikipedia.org")
                        .ok()?
                        .join(href)
                        .ok()?;
                    Some(full_url.to_string())
                })
                .collect::<Vec<_>>()
        };

        match self.args().command {
            crate::args::Command::Wiki { amount: _, link } => {
                if link.is_none() {
                    for link in links {
                        self.add_task(self.spawn_crawl(link, url.clone(), depth + 1))
                            .await;
                    }
                } else {
                    if links.len() as u32 > link.unwrap() {
                        self.add_task(self.spawn_crawl(
                            links[link.unwrap() as usize].clone(),
                            url.clone(),
                            depth + 1,
                        ))
                        .await;
                    } else {
                        info!("No link found on {}", url);
                        return Ok(());
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn spawn_crawl(
        &self,
        url: String,
        from: String,
        depth: usize,
    ) -> tokio::task::JoinHandle<Result<()>> {
        let crawler = self.clone();
        tokio::spawn(async move { crawler.crawl_url(url, from, depth).await })
    }

    fn spawn_add_gephi(
        &self,
        source: String,
        target: String,
        depth: usize,
    ) -> tokio::task::JoinHandle<Result<()>> {
        let crawler = self.clone();
        tokio::spawn(async move { crawler.gephi_add(&source, &target, depth).await })
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
