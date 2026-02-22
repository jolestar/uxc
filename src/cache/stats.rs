//! Cache statistics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cache entries
    pub total_entries: usize,

    /// Total size of cache in bytes
    pub total_size: u64,

    /// Number of cache hits since startup
    pub hits: u64,

    /// Number of cache misses since startup
    pub misses: u64,

    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,

    /// Per-protocol statistics
    pub by_protocol: HashMap<String, ProtocolStats>,
}

impl CacheStats {
    /// Create new cache statistics
    pub fn new() -> Self {
        Self {
            total_entries: 0,
            total_size: 0,
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            by_protocol: HashMap::new(),
        }
    }

    /// Calculate hit rate from hits and misses
    pub fn calculate_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
    }

    /// Get human-readable size string
    pub fn format_size(size: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        if size >= GB {
            format!("{:.2} GB", size as f64 / GB as f64)
        } else if size >= MB {
            format!("{:.2} MB", size as f64 / MB as f64)
        } else if size >= KB {
            format!("{:.2} KB", size as f64 / KB as f64)
        } else {
            format!("{} B", size)
        }
    }

    /// Display statistics in a human-readable format
    pub fn display(&self) -> String {
        let mut output = String::new();
        output.push_str("Cache Statistics:\n");
        output.push_str(&format!("  Total entries: {}\n", self.total_entries));
        output.push_str(&format!(
            "  Total size: {}\n",
            Self::format_size(self.total_size)
        ));
        output.push_str(&format!("  Hits: {}\n", self.hits));
        output.push_str(&format!("  Misses: {}\n", self.misses));
        output.push_str(&format!("  Hit rate: {:.1}%\n", self.hit_rate * 100.0));

        if !self.by_protocol.is_empty() {
            output.push_str("\nBy protocol:\n");
            for (protocol, stats) in &self.by_protocol {
                output.push_str(&format!("  {}:\n", protocol.to_uppercase()));
                output.push_str(&format!("    Entries: {}\n", stats.entries));
                output.push_str(&format!("    Size: {}\n", Self::format_size(stats.size)));
            }
        }

        output
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for a specific protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStats {
    /// Number of cache entries for this protocol
    pub entries: usize,

    /// Total size in bytes for this protocol
    pub size: u64,
}

impl ProtocolStats {
    /// Create new protocol stats
    pub fn new() -> Self {
        Self {
            entries: 0,
            size: 0,
        }
    }
}

impl Default for ProtocolStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_new() {
        let stats = CacheStats::new();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate, 0.0);
    }

    #[test]
    fn test_calculate_hit_rate() {
        let mut stats = CacheStats::new();
        stats.hits = 80;
        stats.misses = 20;
        stats.calculate_hit_rate();
        assert_eq!(stats.hit_rate, 0.8);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(CacheStats::format_size(500), "500 B");
        assert_eq!(CacheStats::format_size(2048), "2.00 KB");
        assert_eq!(CacheStats::format_size(3 * 1024 * 1024), "3.00 MB");
        assert_eq!(CacheStats::format_size(2 * 1024 * 1024 * 1024), "2.00 GB");
    }

    #[test]
    fn test_protocol_stats() {
        let stats = ProtocolStats::new();
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.size, 0);
    }
}
