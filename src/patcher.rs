use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::{io::Write, path::{Path, PathBuf}};
use crate::{config, manifest::{fetch_manifest, PatchFile}};

pub async fn update_client() -> Result<String> {
    let root = std::env::current_dir().context("Não foi possível localizar a pasta do cliente")?;
    let manifest = fetch_manifest(config::MANIFEST_URL).await?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("MODNAROK-Patcher/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let mut updated = 0usize;
    let mut skipped = 0usize;
    for entry in &manifest.files {
        entry.validate()?;
        let destination = safe_destination(&root, &entry.path)?;
        if destination.is_file() && file_matches(&destination, &entry.sha256).await? {
            skipped += 1;
            continue;
        }
        if entry.path.eq_ignore_ascii_case(config::GAME_EXECUTABLE) && game_is_running(config::GAME_EXECUTABLE) {
            bail!("Feche {} antes de atualizar o executável.", config::GAME_EXECUTABLE);
        }
        if let Some(parent) = destination.parent() { tokio::fs::create_dir_all(parent).await?; }
        let temp_path = destination.with_file_name(format!(
            ".{}.modnarok-download",
            destination.file_name().and_then(|s| s.to_str()).unwrap_or("patch-file")
        ));
        download_and_verify(&client, entry, &temp_path).await
            .with_context(|| format!("Falha ao baixar/verificar {}", entry.path))?;

        if destination.exists() {
            // Backup retido para permitir recuperação manual caso necessário.
            let backup = destination.with_file_name(format!(
                "{}.modnarok-backup",
                destination.file_name().and_then(|s| s.to_str()).unwrap_or("patch-file")
            ));
            if backup.exists() { tokio::fs::remove_file(&backup).await?; }
            tokio::fs::rename(&destination, &backup).await
                .with_context(|| format!("Não foi possível criar backup de {}", entry.path))?;
            if let Err(error) = tokio::fs::rename(&temp_path, &destination).await {
                let _ = tokio::fs::rename(&backup, &destination).await;
                return Err(error).with_context(|| format!("Não foi possível instalar {}", entry.path));
            }
        } else {
            tokio::fs::rename(&temp_path, &destination).await
                .with_context(|| format!("Não foi possível instalar {}", entry.path))?;
        }
        updated += 1;
    }
    Ok(format!("Atualização concluída — versão {}. Atualizados: {}; já estavam corretos: {}.", manifest.version, updated, skipped))
}

fn safe_destination(root: &Path, relative: &str) -> Result<PathBuf> {
    let p = Path::new(relative);
    if p.as_os_str().is_empty() || p.is_absolute() || relative.contains(':') ||
       p.components().any(|c| matches!(c, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))) {
        bail!("Caminho fora da instalação recusado: {relative}");
    }
    Ok(root.join(p))
}

async fn file_matches(path: &Path, expected: &str) -> Result<bool> {
    let bytes = tokio::fs::read(path).await?;
    Ok(hex::encode(Sha256::digest(bytes)).eq_ignore_ascii_case(expected))
}

async fn download_and_verify(client: &reqwest::Client, entry: &PatchFile, temporary: &Path) -> Result<()> {
    if temporary.exists() { tokio::fs::remove_file(temporary).await?; }
    let response = client.get(&entry.url).send().await?.error_for_status()?;
    if let (Some(expected), Some(actual)) = (entry.size, response.content_length()) {
        if expected != actual { bail!("Tamanho anunciado não corresponde a {}", entry.path); }
    }
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(temporary).await?;
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        total += chunk.len() as u64;
        if entry.size.map(|expected| total > expected).unwrap_or(false) {
            bail!("Download maior que o esperado: {}", entry.path);
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    drop(file);
    if entry.size.map(|expected| expected != total).unwrap_or(false) {
        let _ = tokio::fs::remove_file(temporary).await;
        bail!("Download incompleto: {}", entry.path);
    }
    if !hex::encode(hasher.finalize()).eq_ignore_ascii_case(&entry.sha256) {
        let _ = tokio::fs::remove_file(temporary).await;
        bail!("SHA-256 não confere para {}; arquivo rejeitado.", entry.path);
    }
    Ok(())
}

fn game_is_running(executable_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        let process_name = executable_name.trim_end_matches(".exe");
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {executable_name}")])
            .output().ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains(&process_name.to_lowercase()))
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    { let _ = executable_name; false }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_parent_traversal() {
        assert!(safe_destination(Path::new("."), "../outside.exe").is_err());
        assert!(safe_destination(Path::new("."), "data/../../outside.exe").is_err());
        assert!(safe_destination(Path::new("."), "data/sprite/file.spr").is_ok());
    }
}
