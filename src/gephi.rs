use reqwest::Client;
use serde_json::json;

pub struct GephiClient {
    base_url: String,
    client: Client,
}

impl GephiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    pub async fn add_node(
        &self,
        node_id: &str,
        label: &str,
        depth: usize,
    ) -> Result<String, reqwest::Error> {
        let body = json!({
            "an": {
                node_id: {
                    "label": label,
                    "depth": depth
                }
            }
        });

        let res = self
            .client
            .post(&format!("{}?operation=updateGraph", self.base_url))
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        Ok(res)
    }

    pub async fn add_edge(
        &self,
        edge_id: &str,
        source: &str,
        target: &str,
        directed: bool,
    ) -> Result<String, reqwest::Error> {
        let body = json!({
            "ae": {
                edge_id: {
                    "source": source,
                    "target": target,
                    "directed": directed
                }
            }
        });

        let res = self
            .client
            .post(&format!("{}?operation=updateGraph", self.base_url))
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        Ok(res)
    }
}
