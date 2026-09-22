use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::jvm::is_safe_flag;
use crate::ram::is_memory_value;
use crate::ServerRecord;

pub fn generate(root: &Path, record: &ServerRecord) -> Result<(), String> {
    let java_home = record
        .java_home
        .as_ref()
        .ok_or("Runtime Java non configurato")?;
    let jar = record.jar_path.as_ref().ok_or("Jar server non configurato")?;
    if !is_memory_value(&record.ram_min) || !is_memory_value(&record.ram_max) {
        return Err("RAM non valida. Usa valori come 2G o 4096M.".into());
    }
    let java = bat_path(root, &crate::java_runtime::java_executable(java_home))?;
    let jar = bat_path(root, jar)?;
    for flag in &record.jvm_flags {
        if !is_safe_flag(flag) {
            return Err(format!("Flag JVM non consentito: {flag}"));
        }
    }
    let flags = if record.jvm_flags.is_empty() {
        String::new()
    } else {
        format!(" {}", record.jvm_flags.join(" "))
    };
    let content = format!(
        "@echo off\r\nif not exist \"eula.txt\" (\r\n    (echo eula=true)> \"eula.txt\"\r\n)\r\n{java} -Xms{ram_min} -Xmx{ram_max}{flags} -jar {jar} nogui\r\nset \"SERVER_EXIT=%ERRORLEVEL%\"\r\nif not \"%SERVER_EXIT%\"==\"0\" (\r\n    echo Server terminato con errore: %SERVER_EXIT%\r\n    pause\r\n    exit /b %SERVER_EXIT%\r\n)\r\nexit /b 0\r\n",
        ram_min = record.ram_min,
        ram_max = record.ram_max,
    );
    std::fs::write(root.join("start.bat"), content).map_err(|error| error.to_string())
}

fn bat_path(root: &Path, path: &Path) -> Result<String, String> {
    let value = path.strip_prefix(root).unwrap_or(path);
    let text = value.to_string_lossy().replace('/', "\\");
    if text.contains('"') || text.contains('\n') || text.contains('\r') {
        return Err("Percorso non valido per start.bat".into());
    }
    Ok(format!("\"{text}\""))
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
    let content = std::fs::read_to_string(root.join("start.bat")).ok()?;
    parse_start_text(root, &content)
}

pub fn parse_start_text(root: &Path, content: &str) -> Option<ParsedStart> {
    let variables = parse_set_variables(content);
    let command_line = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find(|line| line.to_lowercase().contains(" -jar "))
        .map(|line| expand_variables(line, &variables))?;
    let tokens = split_command_line(&command_line);
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

fn parse_set_variables(content: &str) -> HashMap<String, String> {
    let mut variables = HashMap::new();
    for line in content.lines().map(str::trim) {
        let Some(rest) = line.strip_prefix("set ") else {
            continue;
        };
        let trimmed = rest.trim().trim_matches('"');
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        variables.insert(key.to_string(), value.to_string());
    }
    variables
}

fn expand_variables(line: &str, variables: &HashMap<String, String>) -> String {
    let mut expanded = line.to_string();
    for _ in 0..5 {
        let previous = expanded.clone();
        for (key, value) in variables {
            expanded = expanded.replace(&format!("%{key}%"), value);
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
    let mut in_quotes = false;
    for ch in input.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ' ' | '\t' if !in_quotes => {
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
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

pub fn write_eula(root: &Path) -> Result<(), String> {
    std::fs::write(root.join("eula.txt"), "eula=true\r\n").map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parses_generated_and_variable_scripts() {
        let text = r#"@echo off
set "JAVA_HOME_DIR=jdk-21.0.2"
set "RAM_MIN=2G"
set "RAM_MAX=4G"
set "SERVER_JAR=paper-1.21.1-10.jar"
set "RUNTIME_DIR=runtime"
set "JAVA=%RUNTIME_DIR%\%JAVA_HOME_DIR%\bin\java.exe"
"%JAVA%" -Xms%RAM_MIN% -Xmx%RAM_MAX% -XX:+UseG1GC -jar "%SERVER_JAR%" nogui
"#;
        let parsed = parse_start_text(Path::new(r"D:\server"), text).unwrap();
        assert_eq!(parsed.ram_min.as_deref(), Some("2G"));
        assert_eq!(parsed.ram_max.as_deref(), Some("4G"));
        assert_eq!(
            parsed.java_path.unwrap(),
            PathBuf::from(r"D:\server\runtime\jdk-21.0.2\bin\java.exe")
        );
        assert_eq!(
            parsed.jar_path.unwrap(),
            PathBuf::from(r"D:\server\paper-1.21.1-10.jar")
        );
        assert_eq!(parsed.jvm_flags, vec!["-XX:+UseG1GC"]);
    }
}
