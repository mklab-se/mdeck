//! The layout cache: word positions are computed once per (theme, content,
//! size) so the cloud is stable across frames.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};

use super::layout::WordLayout;

/// Cached word positions so layout is stable across frames.
static LAYOUT_CACHE: LazyLock<Mutex<std::collections::HashMap<u64, Vec<WordLayout>>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Every distinct (content, size) pair gets an entry, so window resizes would
/// grow the cache without bound. Past this many entries it is simply reset.
pub(super) const LAYOUT_CACHE_CAP: usize = 64;

pub(super) fn layout_cache()
-> std::sync::MutexGuard<'static, std::collections::HashMap<u64, Vec<WordLayout>>> {
    // A panic while holding the lock must not poison every later frame
    LAYOUT_CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn clear_cache() {
    layout_cache().clear();
}

pub(super) fn cache_key(content: &str, width_bits: u32, height_bits: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    width_bits.hash(&mut hasher);
    height_bits.hash(&mut hasher);
    hasher.finish()
}
