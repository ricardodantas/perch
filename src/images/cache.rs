//! LRU cache for loaded images.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use image::DynamicImage;

/// Maximum number of images to keep in cache
const MAX_CACHE_SIZE: usize = 50;

/// Cache entry for an image
#[derive(Clone)]
pub struct CachedImage {
    /// The decoded image
    pub image: Arc<DynamicImage>,
    /// Last access timestamp (for LRU eviction)
    pub last_access: std::time::Instant,
}

/// Thread-safe image cache
#[derive(Clone)]
pub struct ImageCache {
    /// Raw decoded images
    images: Arc<Mutex<HashMap<String, CachedImage>>>,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCache {
    /// Create a new image cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            images: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Store a decoded image in the cache.
    pub fn insert(&self, url: &str, image: DynamicImage) {
        let mut cache = self.images.lock().unwrap();
        
        // Evict oldest entries if cache is full
        if cache.len() >= MAX_CACHE_SIZE {
            self.evict_oldest(&mut cache);
        }
        
        cache.insert(url.to_string(), CachedImage {
            image: Arc::new(image),
            last_access: std::time::Instant::now(),
        });
    }

    /// Get a decoded image from cache.
    pub fn get(&self, url: &str) -> Option<Arc<DynamicImage>> {
        let mut cache = self.images.lock().unwrap();
        if let Some(entry) = cache.get_mut(url) {
            entry.last_access = std::time::Instant::now();
            Some(Arc::clone(&entry.image))
        } else {
            None
        }
    }

    /// Check if an image is cached.
    pub fn contains(&self, url: &str) -> bool {
        self.images.lock().unwrap().contains_key(url)
    }

    /// Evict the oldest entry from the cache.
    fn evict_oldest(&self, cache: &mut HashMap<String, CachedImage>) {
        if let Some((oldest_key, _)) = cache
            .iter()
            .min_by_key(|(_, v)| v.last_access)
            .map(|(k, v)| (k.clone(), v.last_access))
        {
            cache.remove(&oldest_key);
        }
    }

    /// Clear the entire cache.
    pub fn clear(&self) {
        self.images.lock().unwrap().clear();
    }

    /// Get the number of cached images.
    pub fn len(&self) -> usize {
        self.images.lock().unwrap().len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.images.lock().unwrap().is_empty()
    }
}
