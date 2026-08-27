use ed25519_dalek::{Signature, Verifier, VerifyingKey};

pub fn verify(public_key_hex: &str, signature_hex: &str, timestamp: &str, body: &[u8]) -> bool {
    let Ok(public_key_bytes) = hex::decode(public_key_hex) else { return false };
    let Ok(public_key_array) = public_key_bytes.try_into() else { return false };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&public_key_array) else { return false };

    let Ok(signature_bytes) = hex::decode(signature_hex) else { return false };
    let Ok(signature_array): Result<[u8; 64], _> = signature_bytes.try_into() else { return false };
    let signature = Signature::from_bytes(&signature_array);

    let mut message = timestamp.as_bytes().to_vec();
    message.extend_from_slice(body);

    verifying_key.verify(&message, &signature).is_ok()
}
