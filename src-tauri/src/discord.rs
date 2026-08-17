use reqwest::multipart;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(25);

/// Send a titled command output to Discord as a text file attachment.
/// Mirrors Go sendDiscordFile().
pub async fn send_file(
    webhook_url: &str,
    username: &str,
    title: &str,
    desc: &str,
    filename: &str,
    data: Vec<u8>,
    color: u32,
) -> Result<(), String> {
    let embed = serde_json::json!({
        "title": title,
        "description": desc,
        "color": color,
        "footer": { "text": "Trapline · @foodstampplug" }
    });

    let payload = serde_json::json!({
        "username": username,
        "embeds": [embed]
    });

    let payload_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    let file_part = multipart::Part::bytes(data)
        .file_name(filename.to_string())
        .mime_str("text/plain")
        .map_err(|e| e.to_string())?;

    let form = multipart::Form::new()
        .text("payload_json", payload_str)
        .part("file", file_part);

    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(webhook_url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Discord returned {}: {}", status, body))
    }
}

/// Send a simple text message embed to Discord (no attachment).
/// Used for test webhook and quick embeds.
pub async fn send_embed(
    webhook_url: &str,
    username: &str,
    title: &str,
    description: &str,
    color: u32,
) -> Result<(), String> {
    let embed = serde_json::json!({
        "title": title,
        "description": description,
        "color": color,
        "footer": { "text": "Trapline · @foodstampplug" }
    });

    let payload = serde_json::json!({
        "username": username,
        "embeds": [embed]
    });

    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Discord returned {}: {}", status, body))
    }
}

/// Post a watch alert (content + rich embed array) to a Discord webhook using a
/// BLOCKING client — called from the Watch scheduler thread, not the async
/// runtime. No-op when the webhook is empty.
pub fn send_watch_alert(
    webhook_url: &str,
    username: &str,
    content: &str,
    embeds: serde_json::Value,
) -> Result<(), String> {
    if webhook_url.trim().is_empty() {
        return Ok(());
    }
    let payload = serde_json::json!({
        "username": username,
        "content": content,
        "embeds": embeds,
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        Err(format!("Discord returned {}: {}", status, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_watch_alert_noop_on_empty_webhook() {
        // Empty webhook must short-circuit to Ok without any network call.
        let r = send_watch_alert("", "Trapline", "hi", serde_json::json!([]));
        assert!(r.is_ok());
    }
}
