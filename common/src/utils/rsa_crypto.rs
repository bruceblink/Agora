use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use once_cell::sync::Lazy;
use rsa::pkcs8::EncodePublicKey;
use rsa::rand_core::OsRng;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};

static LOGIN_RSA_PRIVATE_KEY: Lazy<Result<RsaPrivateKey>> = Lazy::new(|| {
    let mut rng = OsRng;
    RsaPrivateKey::new(&mut rng, 2048).context("生成登录 RSA 密钥失败")
});

fn private_key() -> Result<&'static RsaPrivateKey> {
    LOGIN_RSA_PRIVATE_KEY
        .as_ref()
        .map_err(|err| anyhow::anyhow!("{err}"))
}

pub fn login_rsa_public_key_base64() -> Result<String> {
    let public_key = RsaPublicKey::from(private_key()?);
    let der = public_key
        .to_public_key_der()
        .context("导出登录 RSA 公钥失败")?;
    Ok(STANDARD.encode(der.as_bytes()))
}

pub fn decrypt_login_password(value: &str) -> Result<String> {
    let encrypted = STANDARD
        .decode(value)
        .context("登录密码不是有效的 base64 RSA 密文")?;
    let decrypted = private_key()?
        .decrypt(Pkcs1v15Encrypt, &encrypted)
        .context("登录密码 RSA 解密失败")?;
    String::from_utf8(decrypted).context("登录密码不是有效 UTF-8")
}

pub fn decrypt_login_password_or_plain(value: &str) -> String {
    decrypt_login_password(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{decrypt_login_password, login_rsa_public_key_base64};
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use rsa::pkcs8::DecodePublicKey;
    use rsa::rand_core::OsRng;
    use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

    #[test]
    fn public_key_can_encrypt_password_for_private_key() {
        let public_key = login_rsa_public_key_base64().unwrap();
        let der = STANDARD.decode(public_key).unwrap();
        let public_key = RsaPublicKey::from_public_key_der(&der).unwrap();
        let encrypted = public_key
            .encrypt(&mut OsRng, Pkcs1v15Encrypt, b"password123")
            .unwrap();
        let encrypted = STANDARD.encode(encrypted);

        assert_eq!(decrypt_login_password(&encrypted).unwrap(), "password123");
    }
}
