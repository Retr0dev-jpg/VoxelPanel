pub const DEFAULT_JVM_FLAGS: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:+AlwaysPreTouch",
    "-XX:+DisableExplicitGC",
];

pub const JVM_FLAG_CHOICES: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:+DisableExplicitGC",
    "-XX:+AlwaysPreTouch",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UseStringDeduplication",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:G1HeapRegionSize=8M",
    "-XX:G1ReservePercent=20",
    "-XX:InitiatingHeapOccupancyPercent=15",
    "-XX:MaxGCPauseMillis=200",
];

pub fn is_safe_flag(flag: &str) -> bool {
    flag.starts_with("-XX:")
        && flag.len() > 4
        && flag
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '+' | '-' | '=' | '.'))
}

pub fn sanitize_flags(flags: &[String]) -> Result<Vec<String>, String> {
    let mut clean = Vec::new();
    for flag in flags {
        if !is_safe_flag(flag) {
            return Err(format!("Flag JVM non consentito: {flag}"));
        }
        if !clean.iter().any(|existing: &String| existing == flag) {
            clean.push(flag.clone());
        }
    }
    Ok(clean)
}

pub fn default_flags() -> Vec<String> {
    DEFAULT_JVM_FLAGS.iter().map(|flag| (*flag).to_string()).collect()
}
