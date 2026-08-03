//! Source download and filename helpers.

use anyhow::{bail, Result};
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadRecord {
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub bytes: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_type: String,
}

pub(crate) fn is_zero_i64(value: &i64) -> bool {
    *value == 0
}

pub(crate) fn download_one(
    client: &Client,
    raw_url: &str,
    label: &str,
    out_dir: &str,
    max_bytes: i64,
    record: &mut DownloadRecord,
) -> Result<()> {
    let mut response = client
        .get(raw_url)
        .header(
            USER_AGENT,
            format!("SoK-Agent-CLI/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()?;
    if !response.status().is_success() {
        bail!("HTTP {}", response.status().as_u16());
    }

    let content_disposition = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_default();
    let filename = filename_from_headers(
        raw_url,
        content_disposition.as_deref(),
        if content_type.is_empty() {
            None
        } else {
            Some(content_type.as_str())
        },
        label,
    );
    let mut hasher = Sha256::new();
    hasher.update(raw_url.as_bytes());
    let prefix = &hex::encode(hasher.finalize())[..10];
    let output_path = Path::new(out_dir).join(format!("{prefix}-{filename}"));
    let temp_path = output_path.with_extension(format!(
        "{}part",
        output_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!("{ext}."))
            .unwrap_or_default()
    ));
    let mut file = File::create(&temp_path)?;
    let mut seen = 0_i64;
    let mut buffer = [0_u8; 8192];

    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let read = read as i64;
        if seen + read > max_bytes {
            drop(file);
            let _ = fs::remove_file(&temp_path);
            bail!("download exceeded {max_bytes} bytes limit");
        }
        file.write_all(&buffer[..read as usize])?;
        seen += read;
    }
    file.flush()?;
    drop(file);
    fs::rename(&temp_path, &output_path).inspect_err(|_err| {
        let _ = fs::remove_file(&temp_path);
    })?;

    record.status = "downloaded".to_string();
    record.path = output_path.display().to_string();
    record.bytes = seen;
    record.content_type = content_type;
    Ok(())
}

pub fn filename_from_headers(
    raw_url: &str,
    content_disposition: Option<&str>,
    content_type: Option<&str>,
    fallback: &str,
) -> String {
    if let Some(disposition) = content_disposition {
        if let Some(filename) = content_disposition_filename(disposition) {
            return sanitize_filename(&filename);
        }
    }
    if let Ok(parsed) = url::Url::parse(raw_url) {
        if let Some(base) = parsed
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|base| !base.is_empty())
        {
            return sanitize_filename(base);
        }
    }
    if let Some(content_type) = content_type {
        let media_type = content_type
            .split(';')
            .next()
            .unwrap_or(content_type)
            .trim()
            .to_ascii_lowercase();
        if let Some(ext) = common_extension(&media_type) {
            return sanitize_filename(&format!("{fallback}{ext}"));
        }
    }
    sanitize_filename(&format!("{fallback}.bin"))
}

pub(crate) fn content_disposition_filename(disposition: &str) -> Option<String> {
    disposition.split(';').skip(1).find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        if !key.trim().eq_ignore_ascii_case("filename") {
            return None;
        }
        let value = value.trim().trim_matches('"').to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

pub fn common_extension(media_type: &str) -> Option<&'static str> {
    match media_type.to_ascii_lowercase().as_str() {
        "application/pdf" => Some(".pdf"),
        "text/html" => Some(".html"),
        "text/plain" => Some(".txt"),
        "text/markdown" => Some(".md"),
        "application/json" => Some(".json"),
        "text/csv" => Some(".csv"),
        _ => None,
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let mut output = String::new();
    let mut previous_replacement = false;
    for ch in name.trim().chars() {
        let safe = ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-');
        if safe {
            output.push(ch);
            previous_replacement = false;
        } else if !previous_replacement {
            output.push('-');
            previous_replacement = true;
        }
    }
    let trimmed = output.trim_matches(['-', '.']).to_string();
    if trimmed.is_empty() {
        "source.bin".to_string()
    } else {
        trimmed
    }
}

pub fn write_download_record<W: Write>(writer: &mut W, record: &DownloadRecord) -> Result<()> {
    if serde_json::to_writer(&mut *writer, record).is_err() {
        writer.write_all(br#"{"status":"error","reason":"failed to encode download record"}"#)?;
    }
    writer.write_all(b"\n")?;
    Ok(())
}
