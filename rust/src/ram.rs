// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use sysinfo::System;

pub const PRESETS: &[(u32, &str)] = &[
    (512, "0,5 GB"),
    (1024, "1 GB"),
    (2048, "2 GB"),
    (4096, "4 GB"),
    (6144, "6 GB"),
    (8192, "8 GB"),
    (12288, "12 GB"),
    (16384, "16 GB"),
    (24576, "24 GB"),
    (32768, "32 GB"),
    (49152, "48 GB"),
    (65536, "64 GB"),
    (98304, "96 GB"),
    (131072, "128 GB"),
    (262144, "256 GB"),
    (524288, "512 GB"),
    (1048576, "1024 GB"),
];

pub fn ram_value(megabytes: u32) -> String {
    if megabytes.is_multiple_of(1024) {
        format!("{}G", megabytes / 1024)
    } else {
        format!("{megabytes}M")
    }
}

pub fn is_memory_value(value: &str) -> bool {
    if value.len() < 2 {
        return false;
    }
    let (number, suffix) = value.split_at(value.len() - 1);
    !number.is_empty()
        && number.chars().all(|ch| ch.is_ascii_digit())
        && number != "0"
        && matches!(suffix, "G" | "g" | "M" | "m")
}

pub fn suggest() -> (String, String, String) {
    let mut system = System::new();
    system.refresh_memory();
    let total_gb = system.total_memory() / (1024 * 1024 * 1024);
    let (min, max) = match total_gb {
        0..=3 => (1, 1),
        4..=7 => (1, 2),
        8..=15 => (2, 4),
        16..=31 => (2, 6),
        _ => (4, 8),
    };
    (
        format!("{min}G"),
        format!("{max}G"),
        if total_gb == 0 {
            "sconosciuta".to_string()
        } else {
            format!("{total_gb}G")
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_ram_presets() {
        assert_eq!(ram_value(512), "512M");
        assert_eq!(ram_value(2048), "2G");
        assert!(is_memory_value("2G"));
        assert!(is_memory_value("4096M"));
        assert!(!is_memory_value("0G"));
        assert!(!is_memory_value("2"));
    }
}
