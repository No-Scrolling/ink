//! AES-256-GCM in the `ink1.` format: base64(nonce ‖ ciphertext ‖ tag), authenticated with the prefix.
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use base64::{
    Engine,
    alphabet::STANDARD,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};
use ink_runtime::{NativeCall, Responder};

use crate::Failure;

const PREFIX: &str = "ink1.";
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;
/// Android's decoder accepts input with or without padding; encoding always pads.
const BASE64: GeneralPurpose = GeneralPurpose::new(
    &STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

pub(crate) fn call(call: &NativeCall, responder: &Responder) {
    let result = transform(call)
        .map_err(|()| Failure::protocol(format!("Could not {} data", call.operation)));
    match result {
        Ok(text) => responder.resolve(call.id, &serde_json::to_string(&text).expect("strings serialise")),
        Err(failure) => failure.send(responder, call.id),
    };
}

fn transform(call: &NativeCall) -> Result<String, ()> {
    let key = call.payload.get("key").and_then(|value| value.as_str()).ok_or(())?;
    let text = call.payload.get("text").and_then(|value| value.as_str()).ok_or(())?;
    let key = BASE64.decode(key).map_err(|_| ())?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| ())?;
    match call.operation.as_str() {
        "encrypt" => encrypt(&cipher, text),
        "decrypt" => decrypt(&cipher, text),
        _ => Err(()),
    }
}

fn encrypt(cipher: &Aes256Gcm, text: &str) -> Result<String, ()> {
    let mut nonce = [0u8; NONCE_BYTES];
    getrandom::fill(&mut nonce).map_err(|_| ())?;
    let sealed = cipher
        .encrypt(Nonce::from_slice(&nonce), Payload { msg: text.as_bytes(), aad: PREFIX.as_bytes() })
        .map_err(|_| ())?;
    let mut bytes = Vec::with_capacity(NONCE_BYTES + sealed.len());
    bytes.extend_from_slice(&nonce);
    bytes.extend_from_slice(&sealed);
    Ok(format!("{PREFIX}{}", BASE64.encode(bytes)))
}

fn decrypt(cipher: &Aes256Gcm, text: &str) -> Result<String, ()> {
    let encoded = text.strip_prefix(PREFIX).ok_or(())?;
    let bytes = BASE64.decode(encoded).map_err(|_| ())?;
    if bytes.len() < NONCE_BYTES + TAG_BYTES {
        return Err(());
    }
    let (nonce, sealed) = bytes.split_at(NONCE_BYTES);
    let plain = cipher
        .decrypt(Nonce::from_slice(nonce), Payload { msg: sealed, aad: PREFIX.as_bytes() })
        .map_err(|_| ())?;
    String::from_utf8(plain).map_err(|_| ())
}
