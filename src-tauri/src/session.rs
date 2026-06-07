use std::fs;
use std::path::PathBuf;

fn session_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("Trapline").join("session.json")
}

pub fn save(data: &str) -> Result<(), String> {
    let path = session_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, data).map_err(|e| e.to_string())
}

pub fn load() -> Result<String, String> {
    let path = session_path();
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

pub fn clear() -> Result<(), String> {
    let path = session_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}
