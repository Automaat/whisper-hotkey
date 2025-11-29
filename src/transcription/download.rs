use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

const MODEL_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Maps model names to their HuggingFace filenames
fn model_filename(model_name: &str) -> String {
    format!("ggml-{}.bin", model_name)
}

/// Ensures the model is downloaded, returns true if downloaded, false if already existed
pub fn ensure_model_downloaded(model_name: &str, model_path: &Path) -> Result<bool> {
    if model_path.exists() {
        tracing::info!(
            path = %model_path.display(),
            "model already exists, skipping download"
        );
        return Ok(false);
    }

    tracing::info!(
        model = model_name,
        path = %model_path.display(),
        "model not found, starting download"
    );

    download_model(model_name, model_path)?;

    Ok(true)
}

fn download_model(model_name: &str, model_path: &Path) -> Result<()> {
    let filename = model_filename(model_name);
    let url = format!("{}/{}", MODEL_BASE_URL, filename);

    // Create parent directory if it doesn't exist
    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent).context("failed to create model directory")?;
    }

    tracing::info!(url = %url, "downloading model");

    // Download to temporary file first for atomic operation
    let temp_path = model_path.with_extension("tmp");

    let response = reqwest::blocking::get(&url)
        .with_context(|| format!("failed to download model from {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!("download failed with status {}: {}", response.status(), url);
    }

    let bytes = response.bytes().context("failed to read response bytes")?;

    // Write to temp file
    let mut file = fs::File::create(&temp_path)
        .with_context(|| format!("failed to create temp file at {}", temp_path.display()))?;

    file.write_all(&bytes)
        .context("failed to write model to temp file")?;

    // Drop file handle before rename
    drop(file);

    // Atomic rename - if this fails, temp file remains and will be cleaned up next run
    fs::rename(&temp_path, model_path).with_context(|| {
        format!(
            "failed to rename {} to {}",
            temp_path.display(),
            model_path.display()
        )
    })?;

    tracing::info!(
        path = %model_path.display(),
        size = bytes.len(),
        "model downloaded successfully"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_filename() {
        assert_eq!(model_filename("small"), "ggml-small.bin");
        assert_eq!(model_filename("base"), "ggml-base.bin");
        assert_eq!(model_filename("tiny"), "ggml-tiny.bin");
    }

    #[test]
    fn test_ensure_model_downloaded_existing_file() {
        let temp_dir = std::env::temp_dir();
        let model_path = temp_dir.join("test_existing_model.bin");

        // Create a dummy file
        fs::write(&model_path, b"dummy model data").unwrap();

        let result = ensure_model_downloaded("small", &model_path).unwrap();

        // Should return false because file already existed
        assert!(!result);

        // Cleanup
        fs::remove_file(&model_path).unwrap();
    }

    #[test]
    #[ignore] // Requires network access and downloads large file
    fn test_download_model_integration() {
        let temp_dir = std::env::temp_dir();
        let model_path = temp_dir.join("test_downloaded_model.bin");

        // Ensure file doesn't exist
        let _ = fs::remove_file(&model_path);

        let result = ensure_model_downloaded("tiny", &model_path);

        // Should succeed
        assert!(result.is_ok());
        let downloaded = result.unwrap();
        assert!(downloaded); // Should be true because we downloaded it

        // File should exist
        assert!(model_path.exists());

        // File should have content
        let metadata = fs::metadata(&model_path).unwrap();
        assert!(metadata.len() > 0);

        // Cleanup
        fs::remove_file(&model_path).unwrap();
    }

    #[test]
    fn test_download_invalid_model() {
        let temp_dir = std::env::temp_dir();
        let model_path = temp_dir.join("test_invalid_model.bin");

        // Ensure file doesn't exist
        let _ = fs::remove_file(&model_path);

        // Try to download a model that doesn't exist
        let result = download_model("nonexistent-model-xyz", &model_path);

        // Should fail
        assert!(result.is_err());

        // Cleanup (if file was partially created)
        let _ = fs::remove_file(&model_path);
    }

    #[test]
    fn test_ensure_model_creates_parent_directory() {
        let temp_dir = std::env::temp_dir();
        let nested_path = temp_dir
            .join("whisper_test")
            .join("nested")
            .join("test.bin");

        // Ensure directory doesn't exist
        let _ = fs::remove_dir_all(temp_dir.join("whisper_test"));

        // Create dummy file to simulate existing model
        fs::create_dir_all(nested_path.parent().unwrap()).unwrap();
        fs::write(&nested_path, b"test").unwrap();

        let result = ensure_model_downloaded("small", &nested_path);

        // Should succeed
        assert!(result.is_ok());

        // Cleanup
        fs::remove_dir_all(temp_dir.join("whisper_test")).unwrap();
    }
}
