use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use crate::flags::Finding;

/// Small shared store for captured command output so both runner and commands can access it
/// without creating a cycle between their modules.
static CARD_OUTPUT: Lazy<Mutex<HashMap<String, (String, Vec<Finding>)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Store (replace) the output text and findings for a command id.
pub fn store_card_output(id: &str, text: String, findings: Vec<Finding>) {
    let mut store = CARD_OUTPUT.lock().unwrap_or_else(|e| e.into_inner());
    store.insert(id.to_string(), (text, findings));
}

/// Retrieve stored output for an id (clone). Returns (text, findings).
pub fn get_card_output(id: &str) -> (String, Vec<Finding>) {
    let store = CARD_OUTPUT.lock().unwrap_or_else(|e| e.into_inner());
    store.get(id).cloned().unwrap_or_else(|| (String::from("(output not captured)"), vec![]))
}
