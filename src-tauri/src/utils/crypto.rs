use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use md5::{Digest, Md5};
use serde_json::Value;

/// AES-128-ECB encrypt with PKCS7 padding
pub fn aes_ecb_encrypt(data: &[u8], key: &[u8; 16]) -> Vec<u8> {
    let cipher = aes::Aes128::new_from_slice(key).unwrap();
    let block_size = 16;
    let pad_len = block_size - (data.len() % block_size);
    let total_len = data.len() + pad_len;
    let mut buf = vec![0u8; total_len];
    buf[..data.len()].copy_from_slice(data);
    // PKCS7 padding
    for i in data.len()..total_len {
        buf[i] = pad_len as u8;
    }
    for chunk in buf.chunks_mut(block_size) {
        let block = GenericArray::from_mut_slice(chunk);
        cipher.encrypt_block(block);
    }
    buf
}

/// AES-128-ECB decrypt with PKCS7 unpadding
pub fn aes_ecb_decrypt(key: &[u8], data: &[u8]) -> Vec<u8> {
    let cipher = aes::Aes128::new_from_slice(key).unwrap();
    let block_size = 16;
    let mut buf = data.to_vec();
    for chunk in buf.chunks_mut(block_size) {
        let block = GenericArray::from_mut_slice(chunk);
        cipher.decrypt_block(block);
    }
    // Remove PKCS7 padding
    let pad_len = buf.last().copied().unwrap_or(0) as usize;
    if pad_len > 0 && pad_len <= block_size {
        buf.truncate(buf.len() - pad_len);
    }
    buf
}

/// MD5 hex digest
pub fn md5_hex(text: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

const AES_KEY: &[u8; 16] = b"e82ckenh8dichen8";

/// Encrypt params for Netease eapi
pub fn encrypt_params(url: &str, payload: &Value) -> String {
    let url_path = url
        .replace("https://interface3.music.163.com/eapi/", "/api/")
        .replace("http://interface3.music.163.com/eapi/", "/api/");
    let payload_str = serde_json::to_string(payload).unwrap_or_default();
    let digest = md5_hex(&format!("nobody{}use{}md5forencrypt", url_path, payload_str));
    let params = format!("{}-36cd479b6b5-{}-36cd479b6b5-{}", url_path, payload_str, digest);
    let encrypted = aes_ecb_encrypt(params.as_bytes(), AES_KEY);
    hex::encode(encrypted)
}

/// Netease encrypt id for image URLs
pub fn netease_encrypt_id(id_str: &str) -> String {
    let magic = b"3go8&$8*3*3h0k(2)2";
    let id_bytes = id_str.as_bytes();
    let mut result = Vec::with_capacity(id_bytes.len());
    for (i, &b) in id_bytes.iter().enumerate() {
        result.push(b ^ magic[i % magic.len()]);
    }
    let mut hasher = Md5::new();
    hasher.update(&result);
    let hash = hasher.finalize();
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(hash);
    b64.replace('/', "_").replace('+', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_hex() {
        let result = md5_hex("hello");
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_aes_roundtrip() {
        let data = b"hello world! test data 123";
        let encrypted = aes_ecb_encrypt(data, AES_KEY);
        let decrypted = aes_ecb_decrypt(AES_KEY, &encrypted);
        assert_eq!(&decrypted, data);
    }

    #[test]
    fn test_netease_encrypt_id() {
        let enc = netease_encrypt_id("12345");
        assert!(!enc.is_empty());
        assert!(!enc.contains('/'));
        assert!(!enc.contains('+'));
    }
}
