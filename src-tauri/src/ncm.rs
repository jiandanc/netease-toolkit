use crate::utils::crypto::aes_ecb_decrypt;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
pub struct DecryptResult {
    pub file_path: String,
    pub format: String,
    pub name: String,
    pub artists: String,
    pub album: String,
    pub album_pic_url: String,
    pub has_cover_embedded: bool,
}

/// AES-128-ECB keys used by Netease
const CORE_KEY: &[u8; 16] = b"hzHRAmso5kInbaxW";
const MODIFY_KEY: &[u8; 16] = b"#14jkbf!\\]0U<\'(.";


/// Decrypt an .ncm file, output audio and optionally extract cover image
pub fn decrypt_ncm_file(ncm_path: &str, output_dir: &str) -> Result<DecryptResult, String> {
    let path = Path::new(ncm_path);
    if !path.exists() {
        return Err(format!("File not found: {}", ncm_path));
    }

    let mut file = std::fs::File::open(path).map_err(|e| format!("Open file failed: {}", e))?;

    // ---- Magic header check ----
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|_| "Failed to read magic".to_string())?;
    if u32::from_le_bytes(magic) != 0x4E455443 {
        return Err("Not a valid NCM file (invalid magic 1)".to_string());
    }
    file.read_exact(&mut magic).map_err(|_| "Failed to read magic 2".to_string())?;
    if u32::from_le_bytes(magic) != 0x4D414446 {
        return Err("Not a valid NCM file (invalid magic 2)".to_string());
    }

    // ---- Skip 2 bytes version ----
    file.seek(SeekFrom::Current(2)).map_err(|_| "Seek version failed".to_string())?;

    // ---- Read RC4 key length and key data ----
    let mut key_len_buf = [0u8; 4];
    file.read_exact(&mut key_len_buf).map_err(|_| "Read key length failed".to_string())?;
    let key_len = u32::from_le_bytes(key_len_buf) as usize;

    let mut key_data = vec![0u8; key_len];
    file.read_exact(&mut key_data).map_err(|_| "Read key data failed".to_string())?;

    // XOR with 0x64
    for byte in &mut key_data {
        *byte ^= 0x64;
    }

    // AES-128-ECB decrypt with CORE_KEY
    let m_key_data = aes_ecb_decrypt(CORE_KEY, &key_data);

    // ---- Build key box from mKeyData[17:] ----
    let key_box = build_key_box(&m_key_data[17..]);

    // ---- Read metadata ----
    file.read_exact(&mut key_len_buf).map_err(|_| "Read metadata length failed".to_string())?;
    let metadata_len = u32::from_le_bytes(key_len_buf) as usize;

    let mut metadata = NcmMetadata::default();

    if metadata_len > 0 {
        let mut modify_data = vec![0u8; metadata_len];
        file.read_exact(&mut modify_data).map_err(|_| "Read metadata failed".to_string())?;

        // XOR with 0x63
        for byte in &mut modify_data {
            *byte ^= 0x63;
        }

        // Strip "163 key(Don't modify):" prefix (22 bytes)
        let swap_modify = String::from_utf8_lossy(&modify_data[22..]).to_string();

        // Base64 decode
        use base64::Engine;
        let modify_out = base64::engine::general_purpose::STANDARD
            .decode(&swap_modify)
            .map_err(|e| format!("Base64 decode metadata failed: {}", e))?;

        // AES-128-ECB decrypt with MODIFY_KEY
        let modify_decrypt = aes_ecb_decrypt(MODIFY_KEY, &modify_out);

        // Strip "music:" prefix (6 bytes)
        let meta_str = String::from_utf8_lossy(&modify_decrypt[6..]).to_string();
        metadata = parse_ncm_metadata(&meta_str);
    }

    // ---- Skip 5 bytes gap ----
    file.seek(SeekFrom::Current(5)).map_err(|_| "Seek gap failed".to_string())?;

    // ---- Read cover frame ----
    let mut cover_frame_len_buf = [0u8; 4];
    file.read_exact(&mut cover_frame_len_buf).ok();
    let _cover_frame_len = u32::from_le_bytes(cover_frame_len_buf) as i64;

    let mut cover_data_len_buf = [0u8; 4];
    file.read_exact(&mut cover_data_len_buf).ok();
    let cover_data_len = u32::from_le_bytes(cover_data_len_buf) as i64;

    let mut has_cover_embedded = false;

    if cover_data_len > 0 {
        let mut cover_buf = vec![0u8; cover_data_len as usize];
        file.read_exact(&mut cover_buf).ok();
        // Cover is read but not directly returned; we could save it
        has_cover_embedded = true;
    }

    // ---- Decrypt audio data ----
    let mut audio_data = Vec::new();
    let mut buffer = [0u8; 0x8000];

    loop {
        let n = file.read(&mut buffer).map_err(|e| format!("Read audio failed: {}", e))?;
        if n == 0 {
            break;
        }

        // XOR decrypt with keybox
        for i in 0..n {
            let j = ((i + 1) & 0xff) as usize;
            let key_j = key_box[j] as usize;
            let idx = (key_j + key_box[(key_j + j) & 0xff] as usize) & 0xff;
            audio_data.push(buffer[i] ^ key_box[idx]);
        }
    }

    // ---- Determine format by sniffing audio magic bytes ----
    let format = if audio_data.len() > 2 && audio_data[0] == 0x49 && audio_data[1] == 0x44 && audio_data[2] == 0x33 {
        "mp3"
    } else {
        "flac"
    };

    // ---- Write output audio file ----
    let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let output_path = Path::new(output_dir).join(format!("{}.{}", file_stem, format));
    std::fs::create_dir_all(output_dir).map_err(|e| format!("Create output dir failed: {}", e))?;
    std::fs::write(&output_path, &audio_data).map_err(|e| format!("Write audio failed: {}", e))?;

    Ok(DecryptResult {
        file_path: output_path.to_string_lossy().to_string(),
        format: format.to_string(),
        name: metadata.name,
        artists: metadata.artist,
        album: metadata.album,
        album_pic_url: metadata.album_pic_url,
        has_cover_embedded,
    })
}

fn build_key_box(key: &[u8]) -> [u8; 256] {
    let mut key_box = [0u8; 256];
    for i in 0..256 {
        key_box[i] = i as u8;
    }

    let mut swap: u8;
    let mut c: u8;
    let mut last_byte: u8 = 0;
    let mut key_offset: usize = 0;

    for i in 0..256 {
        swap = key_box[i];
        c = swap.wrapping_add(last_byte).wrapping_add(key[key_offset]);
        key_offset += 1;
        if key_offset >= key.len() {
            key_offset = 0;
        }
        key_box[i] = key_box[c as usize];
        key_box[c as usize] = swap;
        last_byte = c;
    }

    key_box
}

#[derive(Default)]
struct NcmMetadata {
    name: String,
    artist: String,
    album: String,
    album_pic_url: String,
    format: String,
    duration: i64,
    bitrate: i64,
}

fn parse_ncm_metadata(json_str: &str) -> NcmMetadata {
    use serde_json::Value;

    let mut meta = NcmMetadata::default();

    if let Ok(val) = serde_json::from_str::<Value>(json_str) {
        if let Some(name) = val.get("musicName").and_then(|v| v.as_str()) {
            meta.name = name.to_string();
        }
        if let Some(album) = val.get("album").and_then(|v| v.as_str()) {
            meta.album = album.to_string();
        }
        if let Some(artists) = val.get("artist").and_then(|v| v.as_array()) {
            let names: Vec<&str> = artists.iter()
                .filter_map(|a| a.as_array())
                .filter_map(|arr| arr.first())
                .filter_map(|v| v.as_str())
                .collect();
            meta.artist = names.join(" / ");
        }
        if let Some(url) = val.get("albumPic").and_then(|v| v.as_str()) {
            meta.album_pic_url = url.to_string();
        }
        if let Some(fmt) = val.get("format").and_then(|v| v.as_str()) {
            meta.format = fmt.to_string();
        }
        if let Some(dur) = val.get("duration").and_then(|v| v.as_i64()) {
            meta.duration = dur;
        }
        if let Some(bit) = val.get("bitrate").and_then(|v| v.as_i64()) {
            meta.bitrate = bit;
        }
    }

    meta
}
