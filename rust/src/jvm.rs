// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use crate::api::settings::JvmPreset;
use crate::{PanelError, PanelResult};

pub const DEFAULT_JVM_FLAGS: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:+AlwaysPreTouch",
    "-XX:+DisableExplicitGC",
];

/// Aikar's flags, the usual recommendation for Paper and derivatives.
pub const AIKAR_FLAGS: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:MaxGCPauseMillis=200",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+DisableExplicitGC",
    "-XX:+AlwaysPreTouch",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:G1HeapRegionSize=8M",
    "-XX:G1ReservePercent=20",
    "-XX:G1HeapWastePercent=5",
    "-XX:G1MixedGCCountTarget=4",
    "-XX:InitiatingHeapOccupancyPercent=15",
    "-XX:G1MixedGCLiveThresholdPercent=90",
    "-XX:G1RSetUpdatingPauseTimePercent=5",
    "-XX:SurvivorRatio=32",
    "-XX:+PerfDisableSharedMem",
    "-XX:MaxTenuringThreshold=1",
    "-Dusing.aikars.flags=https://mcflags.emc.gs",
    "-Daikars.new.flags=true",
];

pub const ZGC_FLAGS: &[&str] = &[
    "-XX:+UseZGC",
    "-XX:+AlwaysPreTouch",
    "-XX:+DisableExplicitGC",
    "-XX:+PerfDisableSharedMem",
];

pub fn preset_flags(preset: JvmPreset) -> Vec<String> {
    let flags: &[&str] = match preset {
        JvmPreset::Aikar => AIKAR_FLAGS,
        JvmPreset::G1 => DEFAULT_JVM_FLAGS,
        JvmPreset::Zgc => ZGC_FLAGS,
        JvmPreset::None => &[],
    };
    flags.iter().map(|flag| (*flag).to_string()).collect()
}

/// Accepts `-XX:` options, `-D` system properties and `-Xss`. Anything that could
/// change what is executed (`-jar`, `-agentpath`, spaces, quotes) is refused.
pub fn is_safe_flag(flag: &str) -> bool {
    let value_ok = |value: &str| {
        !value.is_empty()
            && value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '+' | '-' | '=' | '.' | '_' | '/' | ','))
    };
    if let Some(rest) = flag.strip_prefix("-XX:") {
        return value_ok(rest);
    }
    if let Some(rest) = flag.strip_prefix("-D") {
        let (key, value) = rest.split_once('=').unwrap_or((rest, ""));
        return value_ok(key) && !key.contains('=') && (value.is_empty() || value_ok(value));
    }
    if let Some(size) = flag.strip_prefix("-Xss") {
        return crate::ram::is_memory_value(size) || size.chars().all(|ch| ch.is_ascii_digit());
    }
    false
}

pub fn sanitize_flags(flags: &[String]) -> PanelResult<Vec<String>> {
    let mut clean: Vec<String> = Vec::new();
    for flag in flags.iter().map(|flag| flag.trim()).filter(|flag| !flag.is_empty()) {
        if !is_safe_flag(flag) {
            return Err(PanelError::invalid(format!("Flag JVM non consentito: {flag}")));
        }
        if !clean.iter().any(|existing| existing == flag) {
            clean.push(flag.to_string());
        }
    }
    Ok(clean)
}

pub fn default_flags() -> Vec<String> {
    preset_flags(crate::launcher_settings::current().defaults.jvm_preset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_presets_and_rejects_dangerous_flags() {
        for preset in [JvmPreset::Aikar, JvmPreset::G1, JvmPreset::Zgc] {
            assert!(sanitize_flags(&preset_flags(preset)).is_ok());
        }
        assert!(is_safe_flag("-Dfile.encoding=UTF-8"));
        assert!(is_safe_flag("-Xss4M"));
        assert!(!is_safe_flag("-jar"));
        assert!(!is_safe_flag("-agentpath:/tmp/x.so"));
        assert!(!is_safe_flag("-Dx=\"a b\""));
        assert!(!is_safe_flag("-XX:+Foo bar"));
    }
}
