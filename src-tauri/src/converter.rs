use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Emitter};

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Expand ~ to home directory
fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs_next::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConvertParams {
    pub input_path: String,
    pub output_format: String,
    pub embed_cover: bool,
    pub embed_lyric: bool,
    pub output_dir: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConvertResult {
    pub output_path: String,
    pub success: bool,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Main entry
// ---------------------------------------------------------------------------

pub async fn convert_audio_async(params: &ConvertParams, app_handle: &AppHandle) -> Result<ConvertResult, String> {
    let input = expand_path(&params.input_path);
    if !input.exists() {
        return Ok(ConvertResult {
            output_path: String::new(),
            success: false,
            error: Some(format!("Input file not found: {}", params.input_path)),
        });
    }

    let file_stem = input.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();

    let file_key = params.input_path.clone();
    let output_path = expand_path(&params.output_dir).join(format!("{}.{}", file_stem, params.output_format));
    std::fs::create_dir_all(expand_path(&params.output_dir)).map_err(|e| e.to_string())?;

    // ---- Same-format check: skip re-encoding, just copy & embed metadata ----
    let input_ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if input_ext == params.output_format {
        std::fs::copy(&input, &output_path).map_err(|e| format!("复制文件失败: {}", e))?;
        // Fall through to cover & lyric embedding
    } else {
        // ---- Symphonia: open & probe ----
        let file = File::open(&input).map_err(|e| format!("Failed to open input: {}", e))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let hint = Hint::new();
        let fmt_opts: FormatOptions = Default::default();
        let meta_opts: MetadataOptions = Default::default();
        let dec_opts: AudioDecoderOptions = Default::default();

        let mut reader = symphonia::default::get_probe()
            .probe(&hint, mss, fmt_opts, meta_opts)
            .map_err(|e| format!("Failed to probe audio: {}", e))?;

        let track = reader.default_track(TrackType::Audio)
            .ok_or("No audio track found")?
            .clone();

        let audio_params = track.codec_params
            .as_ref()
            .and_then(|cp| cp.audio())
            .cloned()
            .ok_or("No audio codec parameters")?;

        let sample_rate = audio_params.sample_rate.unwrap_or(44100);
        let num_channels = audio_params.channels.as_ref().map(|c| c.count() as u16).unwrap_or(2);
        let total_frames = track.num_frames.unwrap_or(0);
        let track_id = track.id;

        let mut decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&audio_params, &dec_opts)
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        // ---- Estimate source bitrate for cross-format encoding ----
        let file_size_bytes = std::fs::metadata(&input).map_err(|e| e.to_string())?.len();
        let duration_secs = if total_frames > 0 && sample_rate > 0 {
            total_frames as f64 / sample_rate as f64
        } else {
            0.0
        };

        let estimated_bps = if duration_secs > 0.0 {
            ((file_size_bytes as f64 * 8.0) / duration_secs) as u32
        } else {
            192_000
        };

        // Lossless sources (flac) always cap at 192kbps; otherwise match source quality
        let target_bitrate = if input_ext == "flac" {
            192_000
        } else {
            nearest_cbr_bitrate(estimated_bps)
        };

        // ---- Dispatch by output format ----
        match params.output_format.as_str() {
            "mp3" => convert_to_mp3(
                &mut reader, &mut *decoder, track_id,
                sample_rate, num_channels, total_frames,
                target_bitrate,
                &output_path, &file_key, app_handle,
            )?,
            "m4a" => convert_to_m4a(
                &mut reader, &mut *decoder, track_id,
                sample_rate, num_channels, total_frames,
                target_bitrate,
                &output_path, &file_key, app_handle,
            )?,
            _ => return Ok(ConvertResult {
                output_path: String::new(),
                success: false,
                error: Some(format!("Unsupported output format: {}", params.output_format)),
            }),
        };
    }

    // ---- Migrate metadata tags from source to output (cross-format only) ----
    if input_ext != params.output_format {
        if let Some(meta) = read_metadata_from(&input, &input_ext) {
            let _ = write_metadata_to(&output_path, &params.output_format, &meta);
        }
    }

    // ---- Embed cover & lyrics ----
    if params.embed_cover {
        let parent = input.parent().unwrap_or(Path::new("."));
        for ext in &["jpg", "png"] {
            let cp = parent.join(format!("{}.{}", file_stem, ext));
            if cp.exists() {
                embed_cover_after(&output_path, &cp.to_string_lossy(), &params.output_format)
                    .map_err(|e| format!("嵌入封面失败: {}", e))?;
                break;
            }
        }
    }

    if params.embed_lyric {
        let parent = input.parent().unwrap_or(Path::new("."));
        let lp = parent.join(format!("{}.lrc", file_stem));
        if lp.exists() {
            let lyric_text = std::fs::read_to_string(&lp)
                .map_err(|e| format!("读取歌词文件失败: {}", e))?;
            embed_lyrics_after(&output_path, &lyric_text, &params.output_format)
                .map_err(|e| format!("嵌入歌词失败: {}", e))?;
        }
    }

    let output_str = output_path.to_string_lossy().to_string();

    app_handle.emit("convert-progress", crate::ConvertProgressEvent {
        file_key,
        progress: 100,
        status: "done".to_string(),
        error: None,
        output_path: Some(output_str.clone()),
    }).ok();

    Ok(ConvertResult {
        output_path: output_str,
        success: true,
        error: None,
    })
}

// ---------------------------------------------------------------------------
// MP3 encoding: Symphonia decode -> LAME encode
// ---------------------------------------------------------------------------

fn convert_to_mp3(
    reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    decoder: &mut dyn symphonia::core::codecs::audio::AudioDecoder,
    track_id: u32,
    sample_rate: u32,
    num_channels: u16,
    total_frames: u64,
    target_bitrate: u32,
    output_path: &Path,
    file_key: &str,
    app_handle: &AppHandle,
) -> Result<(), String> {
        use mp3lame_encoder::{Builder, Bitrate, Quality, InterleavedPcm, MonoPcm, FlushNoGap, max_required_buffer_size};

    fn bitrate_to_lame(bps: u32) -> Bitrate {
        match bps {
            v if v <= 128_000 => Bitrate::Kbps128,
            v if v <= 160_000 => Bitrate::Kbps160,
            v if v <= 192_000 => Bitrate::Kbps192,
            v if v <= 256_000 => Bitrate::Kbps256,
            _ => Bitrate::Kbps320,
        }
    }

    let mut lame = Builder::new().ok_or("Failed to create LAME builder")?;
    lame.set_num_channels(num_channels as u8).map_err(|e| format!("LAME channels: {}", e))?;
    lame.set_sample_rate(sample_rate).map_err(|e| format!("LAME sample rate: {}", e))?;
    lame.set_brate(bitrate_to_lame(target_bitrate)).map_err(|e| format!("LAME bitrate: {}", e))?;
    lame.set_quality(Quality::Best).map_err(|e| format!("LAME quality: {}", e))?;
    let mut lame = lame.build().map_err(|e| format!("LAME init: {}", e))?;

    let mut out_file = BufWriter::new(File::create(output_path).map_err(|e| e.to_string())?);
    let mut mp3_buf: Vec<u8> = Vec::with_capacity(64 * 1024);
    let mut decoded_frames: u64 = 0;
    let mut pcm_interleaved: Vec<i16> = Vec::new();

    while let Some(packet) = reader.next_packet().map_err(|e| format!("Read packet: {}", e))? {
        if packet.track_id != track_id {
            continue;
        }

        let audio_buf = match decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("Decode error: {}", e)),
        };

        let frames = audio_buf.frames();
        decoded_frames += frames as u64;

        pcm_interleaved.resize(audio_buf.samples_interleaved(), 0);
        audio_buf.copy_to_slice_interleaved(&mut pcm_interleaved);

        if num_channels == 1 {
            let needed = max_required_buffer_size(pcm_interleaved.len());
            mp3_buf.reserve(needed.saturating_sub(mp3_buf.capacity().saturating_sub(mp3_buf.len())));
            lame.encode_to_vec(MonoPcm(&pcm_interleaved), &mut mp3_buf)
                .map_err(|e| format!("LAME encode: {}", e))?;
        } else {
            let samples_per_ch = pcm_interleaved.len() / 2;
            let needed = max_required_buffer_size(samples_per_ch);
            mp3_buf.reserve(needed.saturating_sub(mp3_buf.capacity().saturating_sub(mp3_buf.len())));
            lame.encode_to_vec(InterleavedPcm(&pcm_interleaved), &mut mp3_buf)
                .map_err(|e| format!("LAME encode: {}", e))?;
        }

        if mp3_buf.len() > 32 * 1024 {
            out_file.write_all(&mp3_buf).map_err(|e| e.to_string())?;
            mp3_buf.clear();
        }

        if total_frames > 0 {
            let progress = ((decoded_frames as f64 / total_frames as f64) * 100.0).min(99.0) as u32;
            app_handle.emit("convert-progress", crate::ConvertProgressEvent {
                file_key: file_key.to_string(),
                progress,
                status: "converting".to_string(),
                error: None,
                output_path: None,
            }).ok();
        }
    }

    let _ = lame.flush_to_vec::<FlushNoGap>(&mut mp3_buf)
        .map_err(|e| format!("LAME flush: {}", e))?;

    if !mp3_buf.is_empty() {
        out_file.write_all(&mp3_buf).map_err(|e| e.to_string())?;
    }
    out_file.flush().map_err(|e| e.to_string())?;

    Ok(())
}

// ---------------------------------------------------------------------------
// M4A encoding: Symphonia decode -> FDK-AAC encode -> MP4 container
// ---------------------------------------------------------------------------

fn convert_to_m4a(
    reader: &mut Box<dyn symphonia::core::formats::FormatReader>,
    decoder: &mut dyn symphonia::core::codecs::audio::AudioDecoder,
    track_id: u32,
    sample_rate: u32,
    num_channels: u16,
    total_frames: u64,
    target_bitrate: u32,
    output_path: &Path,
    file_key: &str,
    app_handle: &AppHandle,
) -> Result<(), String> {
    use fdk_aac::enc::{Encoder as AacEncoder, EncoderParams, BitRate as AacBitRate, ChannelMode, AudioObjectType, Transport};
    use mp4::{Mp4Config, Mp4Writer, TrackConfig as Mp4TrackConfig, MediaConfig, AacConfig, TrackType as Mp4TrackType, ChannelConfig as Mp4ChannelConfig, Mp4Sample, AudioObjectType as Mp4AudioObjectType};

    // Build FDK-AAC encoder
    let channel_mode = if num_channels == 1 { ChannelMode::Mono } else { ChannelMode::Stereo };
    let aac_params = EncoderParams {
        bit_rate: AacBitRate::Cbr(target_bitrate),
        sample_rate,
        transport: Transport::Raw,
        channels: channel_mode,
        audio_object_type: AudioObjectType::Mpeg4LowComplexity,
    };
    let aac_encoder = AacEncoder::new(aac_params)
        .map_err(|e| format!("AAC encoder init: {}", e))?;

    let aac_info = aac_encoder.info()
        .map_err(|e| format!("AAC encoder info: {}", e))?;
    let max_out = aac_info.maxOutBufBytes as usize;

    // Set up MP4 container
    let freq_index = sample_rate_to_freq_index(sample_rate);
    let chan_conf = if num_channels == 1 { Mp4ChannelConfig::Mono } else { Mp4ChannelConfig::Stereo };

    let mp4_config = Mp4Config {
        major_brand: str::parse("M4A ").unwrap(),
        minor_version: 0,
        compatible_brands: vec![
            str::parse("M4A ").unwrap(),
            str::parse("isom").unwrap(),
            str::parse("iso2").unwrap(),
        ],
        timescale: sample_rate,
    };

    let out_file = BufWriter::new(File::create(output_path).map_err(|e| e.to_string())?);
    let mut mp4_writer = Mp4Writer::write_start(out_file, &mp4_config)
        .map_err(|e| format!("MP4 write start: {}", e))?;

    let track_config = Mp4TrackConfig {
        track_type: Mp4TrackType::Audio,
        timescale: sample_rate,
        language: String::from("und"),
        media_conf: MediaConfig::AacConfig(AacConfig {
            bitrate: target_bitrate as u32,
            profile: Mp4AudioObjectType::AacLowComplexity,
            freq_index,
            chan_conf,
        }),
    };
    mp4_writer.add_track(&track_config)
        .map_err(|e| format!("MP4 add track: {}", e))?;

    // Decode -> encode -> write loop
    let mut aac_out_buf = vec![0u8; max_out];
    let mut pcm_interleaved: Vec<i16> = Vec::new();
    let mut decoded_frames: u64 = 0;
    let mut sample_timestamp: u64 = 0;
    let aac_frame_duration = 1024u32;

    while let Some(packet) = reader.next_packet().map_err(|e| format!("Read packet: {}", e))? {
        if packet.track_id != track_id {
            continue;
        }

        let audio_buf = match decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("Decode error: {}", e)),
        };

        let frames = audio_buf.frames();
        decoded_frames += frames as u64;

        pcm_interleaved.resize(audio_buf.samples_interleaved(), 0);
        audio_buf.copy_to_slice_interleaved(&mut pcm_interleaved);

        let mut consumed = 0;
        while consumed < pcm_interleaved.len() {
            let info = aac_encoder.encode(&pcm_interleaved[consumed..], &mut aac_out_buf)
                .map_err(|e| format!("AAC encode: {}", e))?;

            consumed += info.input_consumed;

            if info.output_size > 0 {
                let sample = Mp4Sample {
                    start_time: sample_timestamp,
                    duration: aac_frame_duration,
                    rendering_offset: 0,
                    is_sync: true,
                    bytes: bytes::Bytes::copy_from_slice(&aac_out_buf[..info.output_size]),
                };
                mp4_writer.write_sample(1, &sample)
                    .map_err(|e| format!("MP4 write sample: {}", e))?;
                sample_timestamp += aac_frame_duration as u64;
            }
        }

        if total_frames > 0 {
            let progress = ((decoded_frames as f64 / total_frames as f64) * 100.0).min(99.0) as u32;
            app_handle.emit("convert-progress", crate::ConvertProgressEvent {
                file_key: file_key.to_string(),
                progress,
                status: "converting".to_string(),
                error: None,
                output_path: None,
            }).ok();
        }
    }

    // Flush AAC encoder
    let info = aac_encoder.encode(&[], &mut aac_out_buf)
        .map_err(|e| format!("AAC flush: {}", e))?;
    if info.output_size > 0 {
        let sample = Mp4Sample {
            start_time: sample_timestamp,
            duration: aac_frame_duration,
            rendering_offset: 0,
            is_sync: true,
            bytes: bytes::Bytes::copy_from_slice(&aac_out_buf[..info.output_size]),
        };
        mp4_writer.write_sample(1, &sample)
            .map_err(|e| format!("MP4 write sample: {}", e))?;
    }

    mp4_writer.write_end()
        .map_err(|e| format!("MP4 write end: {}", e))?;

    // Fix mp4 crate SLConfigDescriptor bug for Apple Music compatibility.
    patch_esds_for_apple_compat(output_path)?;

    Ok(())
}

/// Patch the mp4 crate's SLConfigDescriptor bug for Apple Music compatibility.
///
/// The `mp4` crate's SLConfigDescriptor::write_desc writes `size - 1 = 0`
/// as the descriptor length, but the data content is 1 byte. Additionally,
/// the content byte is 0x00 (reserved) instead of 0x02 (MP4 file format).
///
/// Apple Music's AVFoundation parser is strict about these values:
///   - Length must be 1 (not 0) — otherwise downstream parsers (mp4ameta,
///     AVFoundation) fail to read the esds atom correctly.
///   - Content must be 0x02 (MP4 scheme) — using 0x00 causes the AAC
///     decoder to silently produce silence.
///
/// Fix: locate the esds box via its fourcc, then find and correct the
/// SLConfigDescriptor bytes inside it.
fn patch_esds_for_apple_compat(path: &Path) -> Result<(), String> {
    let mut data = std::fs::read(path).map_err(|e| format!("读取M4A文件失败: {}", e))?;

    let esds_marker = b"esds";
    let mut search_start: usize = 0;
    let mut modified = false;

    while search_start + 8 <= data.len() {
        // Find the next "esds" fourcc
        let abs = match data[search_start..].windows(4).position(|w| w == esds_marker) {
            Some(pos) => search_start + pos,
            None => break,
        };

        // The 4 bytes before "esds" hold the box size (big-endian u32).
        if abs >= 4 {
            let box_size = u32::from_be_bytes([
                data[abs - 4], data[abs - 3], data[abs - 2], data[abs - 1],
            ]) as usize;

            // esds box layout: [size:4]["esds":4][version+flags:4]...descriptors...
            let desc_start = abs + 12; // skip size + fourcc + version/flags
            let box_end = abs + box_size;

            if box_end <= data.len() && desc_start < box_end {
                // SLConfigDescriptor (tag 0x06) is the last descriptor in
                // the ES_Descriptor. Scan backwards from the end of the box.
                for i in (desc_start..box_end.saturating_sub(2)).rev() {
                    if data[i] == 0x06 && data[i + 1] == 0x00 && data[i + 2] == 0x00 {
                        data[i + 1] = 0x01; // length: 0 -> 1
                        data[i + 2] = 0x02; // content: 0x00 (reserved) -> 0x02 (MP4)
                        modified = true;
                        break;
                    }
                }
            }
        }

        search_start = abs + 4;
    }

    if modified {
        std::fs::write(path, &data).map_err(|e| format!("写入修补文件失败: {}", e))?;
    }

    Ok(())
}

fn sample_rate_to_freq_index(rate: u32) -> mp4::SampleFreqIndex {
    match rate {
        96000 => mp4::SampleFreqIndex::Freq96000,
        88200 => mp4::SampleFreqIndex::Freq88200,
        64000 => mp4::SampleFreqIndex::Freq64000,
        48000 => mp4::SampleFreqIndex::Freq48000,
        44100 => mp4::SampleFreqIndex::Freq44100,
        32000 => mp4::SampleFreqIndex::Freq32000,
        24000 => mp4::SampleFreqIndex::Freq24000,
        22050 => mp4::SampleFreqIndex::Freq22050,
        16000 => mp4::SampleFreqIndex::Freq16000,
        12000 => mp4::SampleFreqIndex::Freq12000,
        11025 => mp4::SampleFreqIndex::Freq11025,
        8000 => mp4::SampleFreqIndex::Freq8000,
        7350 => mp4::SampleFreqIndex::Freq7350,
        _ => mp4::SampleFreqIndex::Freq44100,
    }
}



// ---------------------------------------------------------------------------
// Bitrate helpers
// ---------------------------------------------------------------------------

/// Map an estimated source bitrate (in bps) to the nearest standard CBR value
fn nearest_cbr_bitrate(estimated_bps: u32) -> u32 {
    const STANDARD_RATES: &[u32] = &[128_000, 160_000, 192_000, 256_000, 320_000];
    let mut best = 192_000;
    let mut min_diff = u32::MAX;
    for &r in STANDARD_RATES {
        let diff = if estimated_bps > r { estimated_bps - r } else { r - estimated_bps };
        if diff < min_diff {
            min_diff = diff;
            best = r;
        }
    }
    best
}
/// Shared metadata struct used for both download embedding and cross-format migration.
pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track: Option<u32>,
    pub year: Option<String>,
    pub duration_secs: Option<u32>,
}

// ---------------------------------------------------------------------------
// Metadata read / write — shared by download and format conversion
// ---------------------------------------------------------------------------

/// Read metadata tags from an existing MP3 or M4A file.
pub fn read_metadata_from(path: &Path, ext: &str) -> Option<AudioMetadata> {
    match ext {
        "mp3" => {
            use id3::TagLike;
            let tag = id3::Tag::read_from_path(path).ok()?;
            Some(AudioMetadata {
                title: tag.title().map(|s| s.to_string()),
                artist: tag.artist().map(|s| s.to_string()),
                album: tag.album().map(|s| s.to_string()),
                track: tag.track(),
                year: tag.year().map(|v| v.to_string()),
                duration_secs: tag.duration(),
            })
        }
        "m4a" => {
            let tag = mp4ameta::Tag::read_from_path(path).ok()?;
            Some(AudioMetadata {
                title: tag.title().map(|s| s.to_string()),
                artist: tag.artist().map(|s| s.to_string()),
                album: tag.album().map(|s| s.to_string()),
                track: tag.track_number().map(|v| v as u32),
                year: tag.year().map(|s| s.to_string()),
                duration_secs: None,
            })
        }
        _ => None,
    }
}

/// Write metadata tags to an existing MP3 or M4A file.
/// Only non-None fields are written; existing tag values are preserved when not provided.
pub fn write_metadata_to(path: &Path, ext: &str, meta: &AudioMetadata) -> Result<(), String> {
    match ext {
        "mp3" => {
            use id3::TagLike;
            let mut tag = id3::Tag::read_from_path(path)
                .unwrap_or_else(|_| id3::Tag::new());
            if let Some(ref v) = meta.title    { tag.set_title(v); }
            if let Some(ref v) = meta.artist   { tag.set_artist(v); }
            if let Some(ref v) = meta.album    { tag.set_album(v); }
            if let Some(v)     = meta.track    { tag.set_track(v); }
            if let Some(ref v) = meta.year     {
                if let Ok(y) = v.parse::<i32>() { tag.set_year(y); }
            }
            if let Some(v)     = meta.duration_secs { tag.set_duration(v); }
            tag.write_to_path(path, id3::Version::Id3v24)
                .map_err(|e| format!("写入 MP3 标签失败: {}", e))
        }
        "m4a" => {
            let mut tag = mp4ameta::Tag::read_from_path(path)
                .map_err(|e| format!("读取 M4A 标签失败: {}", e))?;
            if let Some(ref v) = meta.title    { tag.set_title(v); }
            if let Some(ref v) = meta.artist   { tag.set_artist(v); }
            if let Some(ref v) = meta.album    { tag.set_album(v); }
            if let Some(v)     = meta.track    { tag.set_track_number(v as u16); }
            if let Some(ref v) = meta.year     { tag.set_year(v); }
            tag.write_to_path(path)
                .map_err(|e| format!("写入 M4A 标签失败: {}", e))
        }
        _ => Ok(()),
    }
}

/// Convert a millisecond timestamp to a 4-digit year string.
/// Returns None if the timestamp is zero or invalid.
fn timestamp_ms_to_year(publish_time_ms: i64) -> Option<String> {
    if publish_time_ms <= 0 { return None; }
    // publish_time_ms is milliseconds since Unix epoch.
    // Convert to seconds, then compute year without external crates.
    let secs = publish_time_ms / 1000;
    // Days since 1970-01-01; approximate year via integer division.
    let days = secs / 86400;
    // Civil calendar year from days (sufficiently accurate for year-only).
    let year = 1970 + (days * 400 / 146097) as i64;
    if year > 0 && year < 10000 { Some(year.to_string()) } else { None }
}

/// Embed metadata into an audio file during download.
/// Called from cmd_download_song after the audio bytes are written to disk.
pub fn embed_metadata_after(
    audio_path: &Path,
    title: &str,
    artist: &str,
    album: &str,
    track_number: i32,
    publish_time_ms: i64,
    duration_secs: i64,
    format: &str,
) -> Result<(), String> {
    let meta = AudioMetadata {
        title: Some(title.to_string()),
        artist: Some(artist.to_string()),
        album: Some(album.to_string()),
        track: if track_number > 0 { Some(track_number as u32) } else { None },
        year: timestamp_ms_to_year(publish_time_ms),
        duration_secs: if duration_secs > 0 { Some(duration_secs as u32) } else { None },
    };
    write_metadata_to(audio_path, format, &meta)
}

// ---------------------------------------------------------------------------
// Metadata embedding — returns Result to propagate errors to caller
// ---------------------------------------------------------------------------

fn embed_cover_after(audio_path: &Path, cover_path: &str, format: &str) -> Result<(), String> {
    match format {
        "mp3" => {
            use id3::TagLike;
            let img_data = std::fs::read(cover_path)
                .map_err(|e| format!("读取封面文件失败: {}", e))?;
            let mime_type = if cover_path.ends_with(".png") { "image/png" } else { "image/jpeg" };

            let mut tag = match id3::Tag::read_from_path(audio_path) {
                Ok(t) => t,
                Err(_) => id3::Tag::new(),
            };
            tag.add_frame(id3::frame::Picture {
                mime_type: mime_type.to_string(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: String::new(),
                data: img_data,
            });
            tag.write_to_path(audio_path, id3::Version::Id3v24)
                .map_err(|e| format!("写入 MP3 封面失败: {}", e))?;
            Ok(())
        }
        "m4a" => {
            use mp4ameta::Tag as Mp4Tag;

            let img_data = std::fs::read(cover_path)
                .map_err(|e| format!("读取封面文件失败: {}", e))?;

            let mut tag = Mp4Tag::read_from_path(audio_path)
                .map_err(|e| format!("读取 M4A 标签失败: {}", e))?;

            tag.set_artwork(mp4ameta::Img {
                data: img_data,
                fmt: if cover_path.ends_with(".png") { mp4ameta::ImgFmt::Png } else { mp4ameta::ImgFmt::Jpeg },
            });

            tag.write_to_path(audio_path)
                .map_err(|e| format!("写入 M4A 封面失败: {}", e))?;

            Ok(())
        }
        _ => Ok(()),
    }
}

fn embed_lyrics_after(audio_path: &Path, lyric_text: &str, format: &str) -> Result<(), String> {
    match format {
        "mp3" => {
            use id3::frame::Lyrics;
            use id3::TagLike;

            let mut tag = id3::Tag::read_from_path(audio_path)
                .unwrap_or_else(|_| id3::Tag::new());

            tag.add_frame(Lyrics {
                lang: "chi".to_string(),
                description: String::new(),
                text: lyric_text.to_string(),
            });

            tag.write_to_path(audio_path, id3::Version::Id3v24)
                .map_err(|e| format!("写入 MP3 歌词失败: {}", e))?;

            Ok(())
        }
        "m4a" => {
            use mp4ameta::Tag as Mp4Tag;

            let mut tag = Mp4Tag::read_from_path(audio_path)
                .map_err(|e| format!("读取 M4A 标签失败: {}", e))?;

            tag.set_lyrics(lyric_text);

            tag.write_to_path(audio_path)
                .map_err(|e| format!("写入 M4A 歌词失败: {}", e))?;

            Ok(())
        }
        _ => Ok(()),
    }
}
