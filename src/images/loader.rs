//! Async image loading from URLs.

use image::DynamicImage;
use tokio::sync::mpsc;

use super::ImageCache;

/// Message for the image loader task
#[derive(Debug)]
pub enum LoaderMessage {
    /// Request to load an image
    Load { url: String },
    /// Shutdown the loader
    Shutdown,
}

/// Result of an image load operation
#[derive(Debug, Clone)]
pub enum LoadResult {
    /// Image loaded successfully
    Success { url: String },
    /// Image loading failed
    Failed { url: String, error: String },
}

/// Async image loader that runs in a background task.
pub struct ImageLoader {
    /// Sender to request image loads
    sender: mpsc::UnboundedSender<LoaderMessage>,
    /// Receiver for load results
    result_rx: mpsc::UnboundedReceiver<LoadResult>,
}

impl ImageLoader {
    /// Create a new image loader with a shared cache.
    ///
    /// Spawns a background task to handle image loading.
    pub fn new(cache: ImageCache) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = mpsc::unbounded_channel();

        // Spawn the loader task
        tokio::spawn(loader_task(rx, result_tx, cache));

        Self {
            sender: tx,
            result_rx,
        }
    }

    /// Request an image to be loaded.
    pub fn load(&self, url: &str) {
        let _ = self.sender.send(LoaderMessage::Load {
            url: url.to_string(),
        });
    }

    /// Poll for completed loads (non-blocking).
    pub fn poll_results(&mut self) -> Vec<LoadResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            results.push(result);
        }
        results
    }

    /// Shutdown the loader.
    pub fn shutdown(&self) {
        let _ = self.sender.send(LoaderMessage::Shutdown);
    }
}

/// Background task that handles image loading.
async fn loader_task(
    mut rx: mpsc::UnboundedReceiver<LoaderMessage>,
    result_tx: mpsc::UnboundedSender<LoadResult>,
    cache: ImageCache,
) {
    // Create a reqwest client for downloading images
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    while let Some(msg) = rx.recv().await {
        match msg {
            LoaderMessage::Load { url } => {
                // Skip if already cached
                if cache.contains(&url) {
                    let _ = result_tx.send(LoadResult::Success { url });
                    continue;
                }

                // Download and decode the image
                match download_and_decode(&client, &url).await {
                    Ok(image) => {
                        cache.insert(&url, image);
                        let _ = result_tx.send(LoadResult::Success { url });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load image {url}: {e}");
                        let _ = result_tx.send(LoadResult::Failed {
                            url,
                            error: e.to_string(),
                        });
                    }
                }
            }
            LoaderMessage::Shutdown => {
                tracing::debug!("Image loader shutting down");
                break;
            }
        }
    }
}

/// Download an image from a URL and decode it.
async fn download_and_decode(
    client: &reqwest::Client,
    url: &str,
) -> Result<DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!("Downloading image: {url}");
    
    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()).into());
    }
    
    let bytes = response.bytes().await?;
    
    // Decode the image
    let image = image::load_from_memory(&bytes)?;
    
    // Optionally resize large images to save memory
    let image = resize_if_needed(image);
    
    Ok(image)
}

/// Resize image if it's too large (to save memory and rendering time).
fn resize_if_needed(image: DynamicImage) -> DynamicImage {
    const MAX_DIMENSION: u32 = 800;
    
    let (width, height) = (image.width(), image.height());
    
    if width <= MAX_DIMENSION && height <= MAX_DIMENSION {
        return image;
    }
    
    // Calculate new dimensions maintaining aspect ratio
    let ratio = f64::from(width) / f64::from(height);
    let (new_width, new_height) = if width > height {
        (MAX_DIMENSION, (f64::from(MAX_DIMENSION) / ratio) as u32)
    } else {
        ((f64::from(MAX_DIMENSION) * ratio) as u32, MAX_DIMENSION)
    };
    
    image.resize(new_width, new_height, image::imageops::FilterType::Triangle)
}
