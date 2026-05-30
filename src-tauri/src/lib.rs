pub mod utils;
pub mod commands;
pub mod ncm;
pub mod converter;

use std::time::Instant;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::Emitter;
use std::sync::Mutex;
use std::collections::HashMap;

pub struct AppState {
    pub records_file: Mutex<String>,
    pub cookie: Mutex<String>,
    pub cookie_file: Mutex<String>,
    pub settings_file: Mutex<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SongResult {
    pub id: i64,
    pub name: String,
    pub artists: String,
    pub fee: i64,
    pub album: String,
    pub pic_url: String,

}

#[derive(Serialize, Deserialize, Clone)]
pub struct SongDetail {
    pub id: i64,
    pub name: String,
    pub artists: String,
    pub album: String,
    pub pic_url: String,

    pub duration: i64,
    pub track_number: i32,
    pub publish_time: i64,
    pub lyric: String,
    pub tlyric: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SongUrlResult {
    pub url: String,
    pub size: i64,
    pub r#type: String,
    pub level: String,
}

#[derive(Serialize, Deserialize)]
pub struct PlaylistResult {
    pub id: i64,
    pub name: String,
    pub cover_img_url: String,
    pub creator: String,
    pub track_count: i32,
    pub description: String,
    pub tracks: Vec<SongResult>,
}

#[derive(Serialize, Deserialize)]
pub struct AlbumResult {
    pub id: i64,
    pub name: String,
    pub cover_img_url: String,
    pub artist: String,
    pub songs: Vec<SongResult>,
}

#[derive(Serialize, Deserialize)]
pub struct DownloadProgress {
    pub song_id: i64,
    pub name: String,
    pub progress: u32,
    pub status: String,
}

#[derive(Clone, Serialize)]
pub struct DownloadProgressPayload {
    pub song_id: i64,
    pub progress: u32,
    pub speed: String,
}

#[derive(Serialize, Deserialize)]
pub struct QrLoginResult {
    pub unikey: String,
    pub qr_image_base64: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSongInfo {
    pub name: String,
    pub artists: String,
    pub format: String,
    pub file_size: String,
    pub file_path: String,
    pub has_cover: bool,
    pub has_lyric: bool,
    pub quality: String,
}




fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec > 1_000_000.0 {
        format!("{:.1} MB/s", bytes_per_sec / 1_000_000.0)
    } else if bytes_per_sec > 1_000.0 {
        format!("{:.0} KB/s", bytes_per_sec / 1_000.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}
// ---------------------------------------------------------------------------
// Download records & quality ladder
// ---------------------------------------------------------------------------

const QUALITY_LADDER: &[&str] = &[
    "jymaster", "jyeffect", "sky", "hires", "lossless", "exhigh", "standard"
];

fn read_download_records(records_path: &str) -> HashMap<String, String> {
    let path = std::path::Path::new(records_path);
    if !path.exists() {
        return HashMap::new();
    }
    std::fs::read_to_string(records_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_download_record(records_path: &str, stem: &str, quality: &str) {
    let mut records = read_download_records(records_path);
    records.insert(stem.to_string(), quality.to_string());
    if let Ok(json) = serde_json::to_string(&records) {
        std::fs::write(records_path, json).ok();
    }
}

fn infer_quality_from_ext(ext: &str) -> &str {
    match ext {
        "flac" => "lossless",
        _ => "standard",
    }
}

fn find_quality_index(quality: &str) -> Option<usize> {
    QUALITY_LADDER.iter().position(|&q| q == quality)
}

// ---------------------------------------------------------------------------
// QR Login
// ---------------------------------------------------------------------------

#[tauri::command]
async fn cmd_qr_login_generate(_state: State<'_, AppState>) -> Result<QrLoginResult, String> {
    let api = crate::commands::api::NeteaseApi::new();
    let unikey = api.generate_qr_key().await.map_err(|e| e.to_string())?;

    let qr_url = format!("https://music.163.com/login?codekey={}", unikey);
    let qr_code = qrcode::QrCode::new(qr_url).map_err(|e| e.to_string())?;
    let qr_image = qr_code.render::<image::Luma<u8>>()
        .min_dimensions(300, 300)
        .build();

    let mut png_bytes = std::io::Cursor::new(Vec::new());
    qr_image
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes.into_inner());

    Ok(QrLoginResult { unikey, qr_image_base64: b64 })
}

#[tauri::command]
async fn cmd_qr_login_check(
    unikey: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let api = crate::commands::api::NeteaseApi::new();
    let result = api.check_qr_login(&unikey).await.map_err(|e| e.to_string())?;

    if result.get("code").and_then(|v| v.as_i64()) == Some(803) {
        if let Some(cookie) = result.get("cookie").and_then(|v| v.as_str()) {
            let mut state_cookie = state.cookie.lock().map_err(|e| e.to_string())?;
            *state_cookie = cookie.to_string();
            // Persist cookie to disk so it survives app restart
            let cookie_file = state.cookie_file.lock().map_err(|e| e.to_string())?.clone();
            std::fs::write(&cookie_file, cookie).ok();
        }
    }
    Ok(result)
}

#[tauri::command]
async fn cmd_logout(state: State<'_, AppState>) -> Result<(), String> {
    state.cookie.lock().map_err(|e| e.to_string())?.clear();
    Ok(())
}

#[tauri::command]
async fn cmd_get_login_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let cookie = state.cookie.lock().map_err(|e| e.to_string())?;
    let is_valid = !cookie.is_empty() && cookie.contains("MUSIC_U=");
    Ok(serde_json::json!({
        "isLoggedIn": is_valid,
        "userAvatar": "",
        "userName": ""
    }))
}

#[tauri::command]
async fn cmd_save_cookie(cookie_str: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut cookie = state.cookie.lock().map_err(|e| e.to_string())?;
    *cookie = cookie_str;
    Ok(())
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

#[tauri::command]
async fn cmd_search_music(keywords: String, limit: u32, state: State<'_, AppState>) -> Result<Vec<SongResult>, String> {
    let api = crate::commands::api::NeteaseApi::new();
    let cookie = state.cookie.lock().map_err(|e| e.to_string())?.clone();
    api.search_music(&keywords, limit, &cookie).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_song_detail(song_id: i64, state: State<'_, AppState>) -> Result<SongDetail, String> {
    let api = crate::commands::api::NeteaseApi::new();
    let cookie = state.cookie.lock().map_err(|e| e.to_string())?.clone();
    api.get_song_detail(song_id, &cookie).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_song_url(song_id: i64, quality: String, state: State<'_, AppState>) -> Result<SongUrlResult, String> {
    let api = crate::commands::api::NeteaseApi::new();
    let cookie = state.cookie.lock().map_err(|e| e.to_string())?.clone();
    api.get_song_url(song_id, &quality, &cookie).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_playlist_detail(playlist_id: i64, state: State<'_, AppState>) -> Result<PlaylistResult, String> {
    let api = crate::commands::api::NeteaseApi::new();
    let cookie = state.cookie.lock().map_err(|e| e.to_string())?.clone();
    api.get_playlist_detail(playlist_id, &cookie).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_album_detail(album_id: i64, state: State<'_, AppState>) -> Result<AlbumResult, String> {
    let api = crate::commands::api::NeteaseApi::new();
    let cookie = state.cookie.lock().map_err(|e| e.to_string())?.clone();
    api.get_album_detail(album_id, &cookie).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_resolve_redirect_url(short_url: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(&short_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let final_url = resp.url().to_string();
    Ok(final_url)
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

#[tauri::command]
async fn cmd_download_song(
    song_id: i64, quality: String, download_dir: String,
    download_cover: bool, download_lyric: bool,
    quality_fallback: bool,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let api = crate::commands::api::NeteaseApi::new();
    let cookie = state.cookie.lock().map_err(|e| e.to_string())?.clone();
    let records_path = state.records_file.lock().map_err(|e| e.to_string())?.clone();

    let detail = api.get_song_detail(song_id, &cookie).await.map_err(|e| e.to_string())?;

    // Quality fallback ladder
    let start_idx = find_quality_index(&quality).unwrap_or(6);
    let mut url_info = None;
    let mut _final_quality = String::new();
    if quality_fallback {
        for idx in (0..=start_idx).rev() {
            let q = QUALITY_LADDER[idx];
            match api.get_song_url(song_id, q, &cookie).await {
                Ok(info) if !info.url.is_empty() => {
                    url_info = Some(info);
                    _final_quality = q.to_string();
                    break;
                }
                _ => continue,
            }
        }
    } else {
        match api.get_song_url(song_id, &quality, &cookie).await {
            Ok(info) if !info.url.is_empty() => {
                url_info = Some(info);
                _final_quality = quality.clone();
            }
            _ => {}
        }
    }
    let url_info = url_info.ok_or_else(|| "No available quality found for this song".to_string())?;
    let final_quality_with_type = url_info.level.clone();

    let dir = std::path::PathBuf::from(&download_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let safe_name = detail.name
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");

    let ext = if url_info.r#type.contains("flac") { "flac" }
              else if url_info.r#type.contains("m4a") { "m4a" }
              else { "mp3" };
    let audio_path = dir.join(format!("{}.{}", safe_name, ext));

    // Cookie jar setup (unchanged)
    let cookie_jar = reqwest::cookie::Jar::default();
    if !cookie.is_empty() {
        if let Ok(url_163) = reqwest::Url::parse("https://music.163.com") {
            if let Ok(url_126) = reqwest::Url::parse("https://music.126.net") {
                for part in cookie.split(';') {
                    let kv = part.trim();
                    if kv.is_empty() || !kv.contains('=') { continue; }
                    let set_cookie_163 = format!("{}; Path=/; Domain=.music.163.com", kv);
                    cookie_jar.add_cookie_str(&set_cookie_163, &url_163);
                    let set_cookie_126 = format!("{}; Path=/; Domain=.music.126.net", kv);
                    cookie_jar.add_cookie_str(&set_cookie_126, &url_126);
                }
            }
        }
    }

    let client = reqwest::Client::builder()
        .cookie_provider(std::sync::Arc::new(cookie_jar))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Build client: {}", e))?;

    let resp = client.get(&url_info.url)
        .header("Referer", "https://music.163.com/")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/2.10.2.200154")
        .send().await
        .map_err(|e| format!("Download failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Download returned HTTP {}: {}", status, text.chars().take(200).collect::<String>()));
    }

    let total_size = url_info.size.max(1) as u64;
    let mut downloaded: u64 = 0;
    let mut last_emit_downloaded: u64 = 0;
    let mut last_emit = Instant::now();
    let emit_interval = std::time::Duration::from_millis(200);

    // Write file while streaming
    use std::io::Write;
    let mut file = std::fs::File::create(&audio_path).map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    loop {
        match stream.next().await {
            Some(Ok(chunk)) => {
                file.write_all(&chunk).map_err(|e| e.to_string())?;
                downloaded += chunk.len() as u64;

                let elapsed = last_emit.elapsed();
                if elapsed >= emit_interval {
                    let bytes_since = downloaded - last_emit_downloaded;
                    let speed = bytes_since as f64 / elapsed.as_secs_f64();
                    let progress = ((downloaded * 100) / total_size).min(100) as u32;
                    let speed_str = format_speed(speed);
                    let _ = app_handle.emit("download-progress", DownloadProgressPayload {
                        song_id,
                        progress,
                        speed: speed_str,
                    });
                    last_emit_downloaded = downloaded;
                    last_emit = Instant::now();
                }
            }
            Some(Err(e)) => {
                return Err(format!("Download stream error: {}", e));
            }
            None => break, // stream complete
        }
    }
    // Emit final progress event
    let _ = app_handle.emit("download-progress", DownloadProgressPayload {
        song_id,
        progress: 100,
        speed: String::new(),
    });

    file.flush().map_err(|e| e.to_string())?;
    let file_size = file.metadata().ok().map(|m| m.len()).unwrap_or(0);
    drop(file);

    // Embed metadata
    let _ = crate::converter::embed_metadata_after(
        &audio_path,
        &detail.name,
        &detail.artists,
        &detail.album,
        detail.track_number,
        detail.publish_time,
        detail.duration,
        &ext,
    );

    // Write download record
    write_download_record(&records_path, &safe_name, &final_quality_with_type);

    let mut has_cover = false;
    let mut has_lyric = false;

    if download_cover && !detail.pic_url.is_empty() {
        let cover_path = dir.join(format!("{}.jpg", safe_name));
        if let Ok(resp) = client.get(&detail.pic_url).send().await {
            if let Ok(bytes) = resp.bytes().await {
                std::fs::write(&cover_path, &bytes).ok();
                has_cover = true;
            }
        }
    }

    if download_lyric && !detail.lyric.is_empty() {
        std::fs::write(dir.join(format!("{}.lrc", safe_name)), &detail.lyric).ok();
        has_lyric = true;
    }

    Ok(serde_json::json!({
        "success": true,
        "filePath": audio_path.to_string_lossy().to_string(),
        "hasCover": has_cover, "hasLyric": has_lyric, "fileSize": file_size,
        "quality": final_quality_with_type
    }))
}

// ---------------------------------------------------------------------------
// Local
// ---------------------------------------------------------------------------

#[tauri::command]
async fn cmd_scan_local_dir(dir: String) -> Result<Vec<LocalSongInfo>, String> {
    let dir_path = std::path::PathBuf::from(&dir);
    if !dir_path.exists() { return Ok(Vec::new()); }

    // Load download records if they exist alongside the settings
    let app_data_dir = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("netease-toolkit");
    let records_path = app_data_dir.join("download_records.json");
    let records = read_download_records(&records_path.to_string_lossy());

    let mut songs = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&dir_path)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if !["mp3", "flac", "m4a"].contains(&ext.as_str()) { continue; }

        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let file_size = std::fs::metadata(&path).ok().map(|m| {
            let len = m.len();
            if len > 1024 * 1024 { format!("{:.1}MB", len as f64 / (1024.0 * 1024.0)) }
            else { format!("{:.0}KB", len as f64 / 1024.0) }
        }).unwrap_or_default();

        let has_cover = dir_path.join(format!("{}.jpg", file_stem)).exists()
                     || dir_path.join(format!("{}.png", file_stem)).exists();
        let has_lyric = dir_path.join(format!("{}.lrc", file_stem)).exists();

        // Determine quality: check records first, then infer from ext
        let quality = records.get(&file_stem)
            .cloned()
            .unwrap_or_else(|| infer_quality_from_ext(&ext).to_string());

        // Read name/artist from embedded tags
        let (name, artists) = match crate::converter::read_metadata_from(&path, &ext) {
            Some(meta) => {
                let n = meta.title.unwrap_or_else(|| file_stem.clone());
                let a = meta.artist.unwrap_or_default();
                (n, a)
            }
            None => (file_stem.clone(), String::new()),
        };

        songs.push(LocalSongInfo { name, artists, format: ext, file_size,
            file_path: path.to_string_lossy().to_string(), has_cover, has_lyric, quality });
    }
    Ok(songs)
}

#[tauri::command]
async fn cmd_delete_files(paths: Vec<String>) -> Result<(), String> {
    for p in &paths {
        let path = std::path::Path::new(p);
        if path.exists() { std::fs::remove_file(path).map_err(|e| format!("Delete {} failed: {}", p, e))?; }
        // Also delete associated cover (.jpg, .png) and lyric (.lrc) files
        if let Some(parent) = path.parent() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                for ext in &["jpg", "png", "lrc"] {
                    let companion = parent.join(format!("{}.{}", stem, ext));
                    if companion.exists() {
                        std::fs::remove_file(&companion).ok();
                    }
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn cmd_open_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    { std::process::Command::new("open").arg(&path).spawn().map_err(|e| format!("Failed: {}", e))?; }
    #[cfg(target_os = "windows")]
    { std::process::Command::new("explorer").arg(&path).spawn().map_err(|e| format!("Failed: {}", e))?; }
    #[cfg(target_os = "linux")]
    { std::process::Command::new("xdg-open").arg(&path).spawn().map_err(|e| format!("Failed: {}", e))?; }
    Ok(())
}

// ---------------------------------------------------------------------------
// NCM Decryption
// ---------------------------------------------------------------------------

#[tauri::command]
async fn cmd_decrypt_ncm(ncm_path: String, output_dir: String) -> Result<crate::ncm::DecryptResult, String> {
    crate::ncm::decrypt_ncm_file(&ncm_path, &output_dir)
}

// ---------------------------------------------------------------------------
// Format Conversion
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize)]
pub struct ConvertProgressEvent {
    pub file_key: String,
    pub progress: u32,
    pub status: String,
    pub error: Option<String>,
    pub output_path: Option<String>,
}

#[tauri::command]
async fn cmd_convert_audio(
    params: crate::converter::ConvertParams,
    app_handle: tauri::AppHandle,
) -> Result<crate::converter::ConvertResult, String> {
    crate::converter::convert_audio_async(&params, &app_handle).await
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[tauri::command]
async fn cmd_save_settings(settings: serde_json::Value, state: State<'_, AppState>) -> Result<(), String> {
    let path = state.settings_file.lock().map_err(|e| e.to_string())?;
    std::fs::write(path.as_str(), serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_load_settings(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let path = state.settings_file.lock().map_err(|e| e.to_string())?;
    if std::path::Path::new(path.as_str()).exists() {
        let content = std::fs::read_to_string(path.as_str()).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        let default_download_dir = dirs_next::download_dir()
            .unwrap_or_else(|| dirs_next::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")))
            .join("Music");
        let default_download_path = default_download_dir.to_string_lossy().to_string();
        Ok(serde_json::json!({
            "downloadPath": default_download_path,
            "downloadQuality": "standard",
            "downloadCover": true,
            "qualityFallback": true,
            "downloadLyric": true,
            "maxConcurrentDownloads": 5,
            "maxConcurrentConverts": 5,
            "convertFormat": "mp3",
            "convertOutputPath": default_download_dir.join("Converted").to_string_lossy().to_string()
        }))
    }
}

// ---------------------------------------------------------------------------
// App entry
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_data_dir = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("netease-toolkit");
    std::fs::create_dir_all(&app_data_dir).ok();

    let cookie_path = app_data_dir.join("cookie.txt");
    let settings_path = app_data_dir.join("settings.json");
    let records_path = app_data_dir.join("download_records.json");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            cookie: Mutex::new(std::fs::read_to_string(&cookie_path).unwrap_or_default()),
            cookie_file: Mutex::new(cookie_path.to_string_lossy().to_string()),
            settings_file: Mutex::new(settings_path.to_string_lossy().to_string()),
            records_file: Mutex::new(records_path.to_string_lossy().to_string()),
        })
        .invoke_handler(tauri::generate_handler![
            cmd_qr_login_generate, cmd_qr_login_check, cmd_logout,
            cmd_get_login_status, cmd_save_cookie,
            cmd_search_music, cmd_get_song_detail, cmd_get_song_url,
            cmd_get_playlist_detail, cmd_get_album_detail, cmd_resolve_redirect_url,
            cmd_download_song, cmd_scan_local_dir, cmd_delete_files,
            cmd_open_in_finder,
            cmd_decrypt_ncm, cmd_convert_audio,
            cmd_save_settings, cmd_load_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
