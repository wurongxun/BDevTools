/*! # SSH 会话保险库：主密码 + 凭据加密存储
 *
 * 设计目标：让 `connections` 表里的 `password` / `passphrase` / `advanced_options`
 * 中的代理与跳板机口令不再以明文落盘，同时**不改动任何连接链路**——
 * `ssh_connection.rs` 在写入前加密、读出后解密，`remote_conn.rs` / 前端拿到的
 * 仍然是明文，连接行为完全不变。
 *
 * 为什么不用 OS keyring：Windows Credential Manager / macOS Keychain / Linux
 * Secret Service 三套 API 差异大，且无图形会话（服务、CI、远程桌面）时可能不可用，
 * 加密后的会话文件也无法随应用数据目录整体迁移。这里用「主密码 + PBKDF2(12 万轮)
 * + AES-256-GCM」，密钥只在进程内存，锁屏/重启后必须重新解锁，跨平台行为一致。
 *
 * 兼容性：密文统一带 `bdt1:` 前缀。未设置主密码时所有字段保持明文；启用后只加密
 * 新写入与「加密现有明文」显式迁移的数据；锁定时读出为空串（连接会失败但不会误用
 * 密文去认证），编辑会话会被 423 拦住，避免把密文当明文二次覆盖。
 */
use base64::{engine::general_purpose::STANDARD, Engine as _};
use parking_lot::Mutex;
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU32, path::PathBuf, sync::OnceLock};

const PREFIX: &str = "bdt1:";
const PBKDF2_ITERATIONS: u32 = 120_000;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const VERIFIER_PLAIN: &[u8] = b"BDevTools-SSH-Vault-v1";

/// 进程内密钥。`None` = 锁定；保险库未创建时同样为 `None`。
/// 进程退出即丢失，绝不写进 settings.json 或任何持久化文件。
static KEY: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();

fn key_slot() -> &'static Mutex<Option<Vec<u8>>> {
    KEY.get_or_init(|| Mutex::new(None))
}

#[derive(Serialize, Deserialize)]
struct VaultMeta {
    /// base64(16 字节随机盐)
    salt: String,
    /// 用派生密钥加密 VERIFIER_PLAIN 得到的 `bdt1:` 串，用于校验主密码
    verifier: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    /// 是否已设置主密码（vault.json 存在且可解析）
    pub enabled: bool,
    /// 本次进程是否已解锁
    pub unlocked: bool,
    /// vault.json 的路径，便于用户备份 / 排错
    pub path: String,
}

fn vault_dir() -> PathBuf {
    // 与 sqlite.rs 一致：业务数据目录（卸载时不会被清理），不是 app_data_dir()
    crate::app::app_store::APP.lock().app_dir.join("ssh")
}

fn meta_path() -> PathBuf {
    vault_dir().join("vault.json")
}

fn read_meta() -> Option<VaultMeta> {
    let text = std::fs::read_to_string(meta_path()).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_meta(meta: &VaultMeta) -> Result<(), String> {
    let dir = vault_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建保险库目录失败：{}", e))?;
    let text = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    std::fs::write(meta_path(), text).map_err(|e| format!("写入保险库元数据失败：{}", e))
}

fn random_bytes(len: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; len];
    SystemRandom::new()
        .fill(&mut buf)
        .map_err(|_| "系统随机数不可用".to_string())?;
    Ok(buf)
}

fn derive_key(password: &str, salt: &[u8]) -> Vec<u8> {
    let mut key = vec![0u8; KEY_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
        salt,
        password.as_bytes(),
        &mut key,
    );
    key
}

fn aead_key(raw: &[u8]) -> Result<LessSafeKey, String> {
    let unbound = UnboundKey::new(&AES_256_GCM, raw).map_err(|_| "密钥长度无效".to_string())?;
    Ok(LessSafeKey::new(unbound))
}

fn seal(key: &[u8], plain: &[u8]) -> Result<String, String> {
    let nonce_bytes = random_bytes(NONCE_LEN)?;
    let mut buf = plain.to_vec();
    aead_key(key)?
        .seal_in_place_append_tag(
            Nonce::try_assume_unique_for_key(&nonce_bytes)
                .map_err(|_| "生成 nonce 失败".to_string())?,
            Aad::empty(),
            &mut buf,
        )
        .map_err(|_| "加密失败".to_string())?;
    let mut packed = nonce_bytes;
    packed.extend_from_slice(&buf);
    Ok(format!("{}{}", PREFIX, STANDARD.encode(packed)))
}

fn open(key: &[u8], encoded: &str) -> Result<Vec<u8>, String> {
    let body = encoded
        .strip_prefix(PREFIX)
        .ok_or_else(|| "不是保险库密文".to_string())?;
    let packed = STANDARD.decode(body).map_err(|_| "密文编码损坏".to_string())?;
    if packed.len() <= NONCE_LEN {
        return Err("密文长度不足".to_string());
    }
    let (nonce_bytes, cipher) = packed.split_at(NONCE_LEN);
    let mut owned = cipher.to_vec();
    let plain = aead_key(key)?
        .open_in_place(
            Nonce::try_assume_unique_for_key(nonce_bytes)
                .map_err(|_| "nonce 损坏".to_string())?,
            Aad::empty(),
            &mut owned,
        )
        .map_err(|_| "主密码错误或密文被篡改".to_string())?;
    Ok(plain.to_vec())
}

fn current_key() -> Option<Vec<u8>> {
    key_slot().lock().clone()
}

pub fn is_enabled() -> bool {
    read_meta().is_some()
}

pub fn is_unlocked() -> bool {
    current_key().is_some()
}

pub fn status() -> VaultStatus {
    VaultStatus {
        enabled: is_enabled(),
        unlocked: is_unlocked(),
        path: meta_path().to_string_lossy().to_string(),
    }
}

/// 写会话前调用：保险库已创建但没解锁时禁止写入，
/// 否则用户编辑一条会话就会把空密码（解密失败的结果）写回数据库。
pub fn ensure_writable() -> Result<(), String> {
    if is_enabled() && !is_unlocked() {
        return Err("会话保险库已锁定，请先在「SSH 设置 → 保险库」解锁后再编辑会话".to_string());
    }
    Ok(())
}

/// 设置主密码（已存在则拒绝，避免误把旧密文变成永远解不开的孤儿）。
pub fn setup(password: &str) -> Result<(), String> {
    if password.len() < 6 {
        return Err("主密码至少 6 位".to_string());
    }
    if is_enabled() {
        return Err("保险库已创建，请先解锁；若要取消请先执行「解密并移除」".to_string());
    }
    let salt = random_bytes(SALT_LEN)?;
    let key = derive_key(password, &salt);
    let verifier = seal(&key, VERIFIER_PLAIN)?;
    write_meta(&VaultMeta {
        salt: STANDARD.encode(&salt),
        verifier,
    })?;
    *key_slot().lock() = Some(key);
    Ok(())
}

pub fn unlock(password: &str) -> Result<(), String> {
    let meta = read_meta().ok_or_else(|| "保险库尚未创建".to_string())?;
    let salt = STANDARD
        .decode(&meta.salt)
        .map_err(|_| "保险库元数据损坏".to_string())?;
    let key = derive_key(password, &salt);
    let plain = open(&key, &meta.verifier)?;
    if plain != VERIFIER_PLAIN {
        return Err("主密码错误".to_string());
    }
    *key_slot().lock() = Some(key);
    Ok(())
}

pub fn lock() {
    *key_slot().lock() = None;
}

/// 移除保险库（元数据 + 内存密钥）。调用方必须先把数据库里的密文解密回明文，
/// 否则这些会话将永久无法连接。
pub fn purge() -> Result<(), String> {
    let path = meta_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("删除保险库元数据失败：{}", e))?;
    }
    lock();
    Ok(())
}

fn is_ciphertext(s: &str) -> bool {
    s.starts_with(PREFIX)
}

/// 写库前加密一个凭据字段。未启用保险库时原样返回（明文兼容）。
pub fn encrypt_secret(plain: &str) -> String {
    if plain.is_empty() || is_ciphertext(plain) || !is_enabled() {
        return plain.to_string();
    }
    let Some(key) = current_key() else {
        return plain.to_string();
    };
    match seal(&key, plain.as_bytes()) {
        Ok(s) => s,
        Err(e) => {
            log::error!("ssh vault: encrypt failed: {}", e);
            plain.to_string()
        }
    }
}

/// 读库后解密。锁定 / 密文损坏时返回空串，绝不把密文当密码送去认证。
pub fn decrypt_secret(stored: &str) -> String {
    if stored.is_empty() || !is_ciphertext(stored) {
        return stored.to_string();
    }
    let Some(key) = current_key() else {
        return String::new();
    };
    match open(&key, stored) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(e) => {
            log::warn!("ssh vault: decrypt failed: {}", e);
            String::new()
        }
    }
}

/// `advanced_options` 是一个 JSON 串，里面代理 / 跳板机的口令同样是凭据。
/// 只加解密这几个已知字段，其余原样保留，坏 JSON 直接透传（后端 parse_options 会兜底）。
pub fn transform_advanced(raw: &str, encrypt: bool) -> String {
    if raw.trim().is_empty() {
        return raw.to_string();
    }
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return raw.to_string();
    };
    let transform = |s: &mut serde_json::Value| {
        let Some(text) = s.as_str() else { return };
        if text.is_empty() {
            return;
        }
        let next = if encrypt {
            encrypt_secret(text)
        } else {
            decrypt_secret(text)
        };
        *s = serde_json::Value::String(next);
    };
    if let Some(proxy) = value.get_mut("proxy") {
        if let Some(field) = proxy.get_mut("password") {
            transform(field);
        }
    }
    if let Some(jumps) = value.get_mut("jumpHosts").and_then(|v| v.as_array_mut()) {
        for jump in jumps.iter_mut() {
            for key in ["password", "passphrase"] {
                if let Some(field) = jump.get_mut(key) {
                    transform(field);
                }
            }
        }
    }
    serde_json::to_string(&value).unwrap_or_else(|_| raw.to_string())
}

pub fn encrypt_advanced(raw: &str) -> String {
    transform_advanced(raw, true)
}

pub fn decrypt_advanced(raw: &str) -> String {
    transform_advanced(raw, false)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_roundtrip_and_wrong_password_fails() {
        let salt = b"0123456789abcdef";
        let key = derive_key("correct horse battery", salt);
        let plain = "p@ssw0rd-中文-!@#";
        let cipher = seal(&key, plain.as_bytes()).expect("seal");
        assert!(cipher.starts_with(PREFIX));
        assert_ne!(cipher, plain);
        assert_eq!(open(&key, &cipher).expect("open"), plain.as_bytes());

        let wrong = derive_key("wrong password", salt);
        assert!(open(&wrong, &cipher).is_err(), "错误主密码必须解密失败");

        let tampered = format!("{}A", &cipher[..cipher.len() - 1]);
        assert!(open(&key, &tampered).is_err(), "篡改密文必须被 GCM 校验拦住");

        assert_eq!(open(&key, "not-a-cipher").unwrap_err(), "不是保险库密文");
    }

    #[test]
    fn same_plaintext_seals_to_different_ciphertext() {
        let key = derive_key("pw", b"saltsaltsaltsalt");
        let a = seal(&key, b"secret").unwrap();
        let b = seal(&key, b"secret").unwrap();
        assert_ne!(a, b, "每次 seal 必须用新随机 nonce");
        assert_eq!(open(&key, &a).unwrap(), b"secret");
        assert_eq!(open(&key, &b).unwrap(), b"secret");
    }
}
