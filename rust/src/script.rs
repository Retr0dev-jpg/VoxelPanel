// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::jvm::is_safe_flag;
use crate::ram::is_memory_value;
use crate::{PanelError, PanelResult, ServerRecord};

/// Scripts VoxelPanel recognises when importing a folder, in order of preference.
const SCRIPT_NAMES: &[&str] = &["start.bat", "start.sh", "run.bat", "run.sh", "start.command"];

/// Single source of truth for how a server is launched: used for the real process
/// and for the start script written next to it.
pub fn command_line(record: &ServerRecord) -> PanelResult<(PathBuf, Vec<String>)> {
    let java_home = record.java_home.as_ref().ok_or("Runtime Java non configurato")?;
    let jar = record.jar_path.as_ref().ok_or("Jar server non configurato")?;
    if !is_memory_value(&record.ram_min) || !is_memory_value(&record.ram_max) {
        return Err(PanelError::invalid("RAM non valida. Usa valori come 2G o 4096M."));
    }
    if let Some(flag) = record.jvm_flags.iter().find(|flag| !is_safe_flag(flag)) {
        return Err(PanelError::invalid(format!("Flag JVM non consentito: {flag}")));
    }
    let mut args = vec![format!("-Xms{}", record.ram_min), format!("-Xmx{}", record.ram_max)];
    args.extend(record.jvm_flags.iter().cloned());
    args.push("-jar".into());
    args.push(jar.to_string_lossy().to_string());
    args.push("nogui".into());
    Ok((crate::platform::java_executable(java_home), args))
}

pub fn generate(root: &Path, record: &ServerRecord) -> PanelResult<()> {
    let (java, args) = command_line(record)?;
    let program = relative(root, &java);
    let args: Vec<String> = args.iter().map(|arg| relative(root, Path::new(arg))).collect();
    let content = crate::platform::render_start_script(&crate::platform::ScriptSpec { program: &program, args: &args })?;
    let path = root.join(crate::platform::start_script_name());
    std::fs::write(&path, content)?;
    crate::platform::make_executable(&path)?;
    Ok(())
}

fn relative(root: &Path, value: &Path) -> String {
    value.strip_prefix(root).unwrap_or(value).to_string_lossy().to_string()
}

#[derive(Debug, Clone, Default)]
pub struct ParsedStart {
    pub java_path: Option<PathBuf>,
    pub jar_path: Option<PathBuf>,
    pub ram_min: Option<String>,
    pub ram_max: Option<String>,
    pub jvm_flags: Vec<String>,
}

pub fn parse_start_script(root: &Path) -> Option<ParsedStart> {
    SCRIPT_NAMES
        .iter()
        .filter_map(|name| std::fs::read_to_string(root.join(name)).ok())
        .find_map(|content| parse_start_text(root, &content))
}

pub fn parse_start_text(root: &Path, content: &str) -> Option<ParsedStart> {
    let variables = parse_variables(content);
    let command_line = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.to_lowercase().starts_with("rem "))
        .find(|line| line.contains(" -jar "))
        .map(|line| expand_variables(line, &variables))?;
    let mut tokens = split_command_line(&command_line);
    if tokens.first().is_some_and(|token| token == "exec") {
        tokens.remove(0);
    }
    let mut info = ParsedStart::default();
    if let Some(first) = tokens.first() {
        info.java_path = Some(resolve_script_path(root, first));
    }
    let mut index = 1;
    while index < tokens.len() {
        let token = &tokens[index];
        if let Some(value) = token.strip_prefix("-Xms") {
            info.ram_min = Some(value.to_string());
        } else if let Some(value) = token.strip_prefix("-Xmx") {
            info.ram_max = Some(value.to_string());
        } else if token == "-jar" {
            if let Some(jar) = tokens.get(index + 1) {
                info.jar_path = Some(resolve_script_path(root, jar));
            }
            index += 1;
        } else if token.starts_with("-XX:") {
            info.jvm_flags.push(token.clone());
        }
        index += 1;
    }
    Some(info)
}

/// Collects `set "KEY=value"` (batch) and `KEY=value` / `export KEY=value` (shell).
fn parse_variables(content: &str) -> HashMap<String, String> {
    let mut variables = HashMap::new();
    for line in content.lines().map(str::trim) {
        let rest = line
            .strip_prefix("set ")
            .or_else(|| line.strip_prefix("SET "))
            .or_else(|| line.strip_prefix("export "))
            .unwrap_or(line);
        let trimmed = rest.trim().trim_matches('"');
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.is_empty() || !key.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
            continue;
        }
        variables.insert(key.to_string(), value.trim_matches(['"', '\'']).to_string());
    }
    variables
}

fn expand_variables(line: &str, variables: &HashMap<String, String>) -> String {
    let mut expanded = line.to_string();
    for _ in 0..5 {
        let previous = expanded.clone();
        for (key, value) in variables {
            expanded = expanded
                .replace(&format!("%{key}%"), value)
                .replace(&format!("${{{key}}}"), value)
                .replace(&format!("${key}"), value);
        }
        if expanded == previous {
            break;
        }
    }
    expanded
}

fn split_command_line(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in input.chars() {
        match (ch, quote) {
            ('"' | '\'', None) => quote = Some(ch),
            (c, Some(open)) if c == open => quote = None,
            (' ' | '\t', None) => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn resolve_script_path(root: &Path, value: &str) -> PathBuf {
    let path = crate::platform::native_path(value);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

pub fn write_eula(root: &Path) -> PanelResult<()> {
    std::fs::write(root.join("eula.txt"), "eula=true\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_batch_scripts_with_variables() {
        let text = r#"@echo off
set "JAVA_HOME_DIR=jdk-21.0.2"
set "RAM_MIN=2G"
set "RAM_MAX=4G"
set "SERVER_JAR=paper-1.21.1-10.jar"
set "RUNTIME_DIR=runtime"
set "JAVA=%RUNTIME_DIR%\%JAVA_HOME_DIR%\bin\java.exe"
"%JAVA%" -Xms%RAM_MIN% -Xmx%RAM_MAX% -XX:+UseG1GC -jar "%SERVER_JAR%" nogui
"#;
        let root = Path::new("server");
        let parsed = parse_start_text(root, text).unwrap();
        assert_eq!(parsed.ram_min.as_deref(), Some("2G"));
        assert_eq!(parsed.ram_max.as_deref(), Some("4G"));
        assert_eq!(
            parsed.java_path.unwrap(),
            root.join("runtime").join("jdk-21.0.2").join("bin").join("java.exe")
        );
        assert_eq!(parsed.jar_path.unwrap(), root.join("paper-1.21.1-10.jar"));
        assert_eq!(parsed.jvm_flags, vec!["-XX:+UseG1GC"]);
    }

    #[test]
    fn parses_shell_scripts() {
        let text = "#!/usr/bin/env sh\nJAR='paper 1.21.jar'\nexec 'runtime/jdk-21/bin/java' -Xms1G -Xmx3G -jar \"$JAR\" nogui\n";
        let root = Path::new("server");
        let parsed = parse_start_text(root, text).unwrap();
        assert_eq!(parsed.ram_max.as_deref(), Some("3G"));
        assert_eq!(parsed.java_path.unwrap(), root.join("runtime").join("jdk-21").join("bin").join("java"));
        assert_eq!(parsed.jar_path.unwrap(), root.join("paper 1.21.jar"));
    }

    #[test]
    fn generated_script_round_trips() {
        let root = std::env::temp_dir().join(format!("voxel-script-{}", uuid::Uuid::new_v4()));
        let home = root.join("runtime").join("jdk-21");
        std::fs::create_dir_all(&home).unwrap();
        let record = ServerRecord {
            id: "x".into(),
            name: "x".into(),
            root: root.clone(),
            paper_version: None,
            java_major: Some(21),
            java_home: Some(home.clone()),
            jar_path: Some(root.join("paper-1.21.1-1.jar")),
            ram_min: "1G".into(),
            ram_max: "2G".into(),
            jvm_flags: vec!["-XX:+UseG1GC".into()],
            eula_accepted: true,
            created_unix: 0,
        };
        generate(&root, &record).unwrap();
        let parsed = parse_start_script(&root).unwrap();
        assert_eq!(parsed.java_path.unwrap(), crate::platform::java_executable(&home));
        assert_eq!(parsed.jar_path.unwrap(), root.join("paper-1.21.1-1.jar"));
        assert_eq!(parsed.ram_min.as_deref(), Some("1G"));
        assert_eq!(parsed.jvm_flags, vec!["-XX:+UseG1GC"]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
