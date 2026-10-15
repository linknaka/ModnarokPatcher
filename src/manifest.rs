use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub files: Vec<PatchFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PatchFile {
    /// Caminho relativo à pasta do cliente, por exemplo "data/sprite/exemplo.spr".
    pub path: String,
    /// URL HTTPS absoluta.
    pub url: String,
    /// SHA-256 esperado, em 64 caracteres hexadecimais.
    pub sha256: String,
    pub size: Option<u64>,
}

impl PatchFile {
    pub fn validate(&self) -> Result<()> {
        let p = Path::new(&self.path);
        if p.as_os_str().is_empty() || p.is_absolute() || self.path.contains(':') ||
           p.components().any(|c| matches!(c, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))) {
            bail!("Caminho inseguro no manifesto: {}", self.path);
        }
        if !self.url.starts_with("https://") {
            bail!("O download precisa usar HTTPS: {}", self.path);
        }
        if self.sha256.len() != 64 || hex::decode(&self.sha256).is_err() {
            bail!("SHA-256 inválido para {}", self.path);
        }
        Ok(())
    }
}

pub async fn fetch_manifest(url: &str) -> Result<Manifest> {
    if !url.starts_with("https://") || url.contains("SEU-DOMINIO-AQUI") {
        bail!("Configure MANIFEST_URL em src/config.rs antes de testar downloads.");
    }
    let response = reqwest::Client::builder()
        .user_agent(concat!("MODNAROK-Patcher/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()?
        .get(url)
        .send().await
        .context("Não foi possível acessar o manifesto")?
        .error_for_status()
        .context("O servidor retornou erro ao buscar o manifesto")?;
    let manifest: Manifest = response.json().await.context("Manifesto JSON inválido")?;
    if manifest.version.trim().is_empty() { bail!("Manifesto sem versão"); }
    let mut seen = std::collections::HashSet::new();
    for entry in &manifest.files {
        entry.validate()?;
        if !seen.insert(entry.path.to_ascii_lowercase()) {
            bail!("Caminho duplicado no manifesto: {}", entry.path);
        }
    }
    Ok(manifest)
}
