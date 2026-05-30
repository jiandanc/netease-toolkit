use crate::{AlbumResult, PlaylistResult, SongDetail, SongResult, SongUrlResult};
use crate::utils::crypto::{encrypt_params, netease_encrypt_id};
use rand::Rng;
use reqwest::Client;
use serde_json::{json, Value};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/2.10.2.200154";
const REFERER: &str = "https://music.163.com/";

const SONG_URL_V1: &str = "https://interface3.music.163.com/eapi/song/enhance/player/url/v1";
const SONG_DETAIL_V3: &str = "https://interface3.music.163.com/api/v3/song/detail";
const LYRIC_API: &str = "https://interface3.music.163.com/api/song/lyric";
const SEARCH_API: &str = "https://music.163.com/api/cloudsearch/pc";
const PLAYLIST_DETAIL_API: &str = "https://music.163.com/api/v6/playlist/detail";
const ALBUM_DETAIL_API: &str = "https://music.163.com/api/v1/album/";
const QR_UNIKEY_API: &str = "https://interface3.music.163.com/eapi/login/qrcode/unikey";
const QR_LOGIN_API: &str = "https://interface3.music.163.com/eapi/login/qrcode/client/login";

pub struct NeteaseApi {
    client: Client,
}

impl NeteaseApi {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    fn default_config() -> Value {
        let request_id: u64 = rand::thread_rng().gen_range(20000000..30000000);
        json!({
            "os": "pc",
            "appver": "",
            "osver": "",
            "deviceId": "pyncm!",
            "requestId": request_id.to_string()
        })
    }

    fn headers() -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("User-Agent", reqwest::header::HeaderValue::from_static(USER_AGENT));
        headers.insert("Referer", reqwest::header::HeaderValue::from_static(REFERER));
        headers
    }

    /// Generate QR unikey
    pub async fn generate_qr_key(&self) -> Result<String, anyhow::Error> {
        let config = Self::default_config();
        let payload = json!({
            "type": 1,
            "header": config
        });
        let params = encrypt_params(QR_UNIKEY_API, &payload);
        let resp = self
            .client
            .post(QR_UNIKEY_API)
            .headers(Self::headers())
            .form(&[("params", &params)])
            .send()
            .await?;
        let result: Value = resp.json().await?;
        result
            .get("unikey")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to get unikey"))
    }

    /// Check QR login status
    pub async fn check_qr_login(&self, unikey: &str) -> Result<Value, anyhow::Error> {
        let config = Self::default_config();
        let payload = json!({
            "key": unikey,
            "type": 1,
            "header": config
        });
        let params = encrypt_params(QR_LOGIN_API, &payload);
        let resp = self
            .client
            .post(QR_LOGIN_API)
            .headers(Self::headers())
            .form(&[("params", &params)])
            .send()
            .await?;

        // Capture headers before consuming the response body
        let cookie_headers: Vec<String> = resp
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();

        let text = resp.text().await?;
        let result: Value = serde_json::from_str(&text)?;

        let mut output = json!({
            "code": result.get("code").and_then(|v| v.as_i64()).unwrap_or(-1)
        });

        if result.get("code").and_then(|v| v.as_i64()) == Some(803) {
            // Parse set-cookie headers: extract key=value before first ';'
            let cookies: Vec<String> = cookie_headers
                .iter()
                .filter_map(|h| h.split(';').next().map(|kv| kv.trim().to_string()))
                .filter(|kv| {
                    !kv.contains('=') || !["path", "domain", "expires", "max-age", "samesite", "httponly", "secure"]
                        .iter().any(|attr| kv.to_lowercase().starts_with(&format!("{}=", attr)))
                })
                .collect();
            if !cookies.is_empty() {
                output["cookie"] = json!(cookies.join("; "));
            }
        }

        Ok(output)
    }

    /// Search music
    pub async fn search_music(&self, keywords: &str, limit: u32, cookie: &str) -> Result<Vec<SongResult>, anyhow::Error> {
        let form_data = [
            ("s", keywords),
            ("type", "1"),
            ("limit", &limit.to_string()),
        ];

        let mut req = self
            .client
            .post(SEARCH_API)
            .headers(Self::headers());
        if !cookie.is_empty() {
            req = req.header("Cookie", cookie);
        }
        let resp = req.form(&form_data).send().await?;

        let result: Value = resp.json().await?;
        let mut songs = Vec::new();

        if let Some(song_list) = result
            .get("result")
            .and_then(|r| r.get("songs"))
            .and_then(|s| s.as_array())
        {
            for item in song_list {
                let artists = item
                    .get("ar")
                    .and_then(|ar| ar.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .unwrap_or_default();

                let album_name = item
                    .get("al")
                    .and_then(|al| al.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let pic_url = item
                    .get("al")
                    .and_then(|al| al.get("picUrl"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("");

                songs.push(SongResult {
                    id: item.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    artists,
                    album: album_name.to_string(),
                    pic_url: pic_url.to_string(),
                    fee: item.get("fee").and_then(|v| v.as_i64()).unwrap_or(0),
                });
            }
        }

        Ok(songs)
    }

    /// Get song detail with lyric
    pub async fn get_song_detail(&self, song_id: i64, cookie: &str) -> Result<SongDetail, anyhow::Error> {
        // Get song info
        let data = json!({"c": serde_json::to_string(&vec![json!({"id": song_id, "v": 0})]).unwrap_or_default()});
        let mut req = self
            .client
            .post(SONG_DETAIL_V3)
            .headers(Self::headers());
        if !cookie.is_empty() {
            req = req.header("Cookie", cookie);
        }
        let resp = req.form(&data).send().await?;
        let result: Value = resp.json().await?;

        let song = result
            .get("songs")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| anyhow::anyhow!("Song not found"))?;

        let artists = song
            .get("ar")
            .and_then(|ar| ar.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_default();

        let pic_url = song
            .get("al")
            .and_then(|al| al.get("picUrl"))
            .and_then(|p| p.as_str())
            .unwrap_or("");

        // Get lyric
        let lyric_form = [
            ("id", &song_id.to_string()),
            ("cp", &"false".to_string()),
            ("tv", &"0".to_string()),
            ("lv", &"0".to_string()),
            ("rv", &"0".to_string()),
            ("kv", &"0".to_string()),
            ("yv", &"0".to_string()),
            ("ytv", &"0".to_string()),
            ("yrv", &"0".to_string()),
        ];

        let lyric_resp = self
            .client
            .post(LYRIC_API)
            .headers(Self::headers())
            .form(&lyric_form)
            .send()
            .await?;
        let lyric_result: Value = lyric_resp.json().await?;

        let lyric = lyric_result
            .get("lrc")
            .and_then(|l| l.get("lyric"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tlyric = lyric_result
            .get("tlyric")
            .and_then(|l| l.get("lyric"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(SongDetail {
            id: song_id,
            name: song.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            artists,
            album: song.get("al").and_then(|al| al.get("name")).and_then(|n| n.as_str()).unwrap_or("").to_string(),
            pic_url: pic_url.to_string(),
            duration: song.get("dt").and_then(|v| v.as_i64()).unwrap_or(0) / 1000,
            track_number: song.get("no").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(0),
            publish_time: song.get("publishTime").and_then(|v| v.as_i64()).unwrap_or(0),
            lyric,
            tlyric,
        })
    }

    /// Get song download URL
    pub async fn get_song_url(&self, song_id: i64, quality: &str, cookie: &str) -> Result<SongUrlResult, anyhow::Error> {
        let config = Self::default_config();
        let mut payload = json!({
            "ids": [song_id],
            "level": quality,
            "encodeType": "flac",
            "header": config
        });
        if quality == "sky" {
            payload["immerseType"] = json!("c51");
        }

        let params = encrypt_params(SONG_URL_V1, &payload);

        let resp = self
            .client
            .post(SONG_URL_V1)
            .headers(Self::headers())
            .header("Cookie", cookie)
            .form(&[("params", &params)])
            .send()
            .await?;

        let result: Value = resp.json().await?;

        if result.get("code").and_then(|v| v.as_i64()) != Some(200) {
            return Err(anyhow::anyhow!("Failed to get song URL"));
        }

        let data = result
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| anyhow::anyhow!("No URL data"))?;

        Ok(SongUrlResult {
            url: data.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            size: data.get("size").and_then(|v| v.as_i64()).unwrap_or(0),
            r#type: data.get("type").and_then(|v| v.as_str()).unwrap_or("mp3").to_string(),
            level: data.get("level").and_then(|v| v.as_str()).unwrap_or(quality).to_string(),
        })
    }

    /// Get playlist detail
    pub async fn get_playlist_detail(&self, playlist_id: i64, _cookie: &str) -> Result<PlaylistResult, anyhow::Error> {
        let form_data = [("id", playlist_id.to_string())];
        let resp = self
            .client
            .post(PLAYLIST_DETAIL_API)
            .headers(Self::headers())
            .form(&form_data)
            .send()
            .await?;
        let result: Value = resp.json().await?;

        let playlist = result
            .get("playlist")
            .ok_or_else(|| anyhow::anyhow!("Playlist not found"))?;

        let mut tracks = Vec::new();
        if let Some(track_ids) = playlist.get("trackIds").and_then(|t| t.as_array()) {
            for chunk in track_ids.chunks(100) {
                let id_list: Vec<Value> = chunk
                    .iter()
                    .filter_map(|t| t.get("id").and_then(|id| id.as_i64()))
                    .map(|id| json!({"id": id, "v": 0}))
                    .collect();

                let song_data = serde_json::to_string(&id_list).unwrap_or_default();
                let form = [("c", song_data.as_str())];
                if let Ok(song_resp) = self
                    .client
                    .post(SONG_DETAIL_V3)
                    .headers(Self::headers())
                    .form(&form)
                    .send()
                    .await
                {
                    if let Ok(song_result) = song_resp.json::<Value>().await {
                        if let Some(songs) = song_result.get("songs").and_then(|s| s.as_array()) {
                            for s in songs {
                                let artists = s
                                    .get("ar")
                                    .and_then(|ar| ar.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                                            .collect::<Vec<_>>()
                                            .join("/")
                                    })
                                    .unwrap_or_default();
                                tracks.push(SongResult {
                                    id: s.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                                    name: s.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    artists,
                                    album: s.get("al").and_then(|al| al.get("name")).and_then(|n| n.as_str()).unwrap_or("").to_string(),
                                    pic_url: s.get("al").and_then(|al| al.get("picUrl")).and_then(|p| p.as_str()).unwrap_or("").to_string(),
                                    fee: s.get("fee").and_then(|v| v.as_i64()).unwrap_or(0),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(PlaylistResult {
            id: playlist.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            name: playlist.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cover_img_url: playlist.get("coverImgUrl").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            creator: playlist.get("creator").and_then(|c| c.get("nickname")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            track_count: playlist.get("trackCount").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(0),
            description: playlist.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            tracks,
        })
    }

    /// Get album detail
    pub async fn get_album_detail(&self, album_id: i64, _cookie: &str) -> Result<AlbumResult, anyhow::Error> {
        let url = format!("{}{}", ALBUM_DETAIL_API, album_id);
        let resp = self
            .client
            .get(&url)
            .headers(Self::headers())
            .send()
            .await?;
        let result: Value = resp.json().await?;

        let album = result
            .get("album")
            .ok_or_else(|| anyhow::anyhow!("Album not found"))?;

        let mut songs = Vec::new();
        if let Some(song_list) = result.get("songs").and_then(|s| s.as_array()) {
            for s in song_list {
                let artists = s
                    .get("ar")
                    .and_then(|ar| ar.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .unwrap_or_default();
                songs.push(SongResult {
                    id: s.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    name: s.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    artists,
                    album: album.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    pic_url: s.get("al").and_then(|al| {
                        let pic = al.get("pic");
                        pic.and_then(|p| p.as_i64())
                            .map(|pid| {
                                let enc = netease_encrypt_id(&pid.to_string());
                                format!("https://p3.music.126.net/{}/{}.jpg?param=300y300", enc, pid)
                            })
                            .or_else(|| al.get("picUrl").and_then(|p| p.as_str()).map(|s| s.to_string()))
                    }).unwrap_or_default(),
                    fee: s.get("fee").and_then(|v| v.as_i64()).unwrap_or(0),
                });
            }
        }

        Ok(AlbumResult {
            id: album.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            name: album.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cover_img_url: album.get("pic").and_then(|p| p.as_i64()).map(|pid| {
                let enc = netease_encrypt_id(&pid.to_string());
                format!("https://p3.music.126.net/{}/{}.jpg?param=300y300", enc, pid)
            }).unwrap_or_default(),
            artist: album.get("artist").and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            songs,
        })
    }
}
