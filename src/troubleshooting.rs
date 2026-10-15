use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HelpArticle {
    pub id: String,
    pub title: String,
    pub symptoms: String,
    pub solution: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub links: Vec<HelpLink>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HelpLink {
    pub label: String,
    pub url: String,
}

pub async fn fetch_articles() -> Result<Vec<HelpArticle>> {
    let url = crate::config::TROUBLESHOOTING_URL;
    if !url.starts_with("https://") || url.contains("SEU-DOMINIO-AQUI") {
        bail!("Configure TROUBLESHOOTING_URL em src/config.rs para ativar a central online.");
    }
    let articles: Vec<HelpArticle> = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?
        .get(url)
        .send().await
        .context("Erro ao acessar a central de soluções")?
        .error_for_status()?
        .json().await
        .context("JSON de troubleshooting inválido")?;
    for article in &articles {
        for link in &article.links {
            if !link.url.starts_with("https://") {
                bail!("Link de solução precisa usar HTTPS: {}", link.label);
            }
        }
    }
    Ok(articles)
}
