use crate::db::{Db, Worker};
use rand::Rng;
use sha2::{Digest, Sha256};

pub fn hash_token(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn issue_token(db: &Db, discord_user_id: &str) -> anyhow::Result<String> {
    let raw = generate_token();
    let hash = hash_token(&raw);
    db.insert_worker(discord_user_id, &hash)?;
    Ok(raw)
}

pub fn authenticate(db: &Db, presented_token: &str) -> anyhow::Result<Option<Worker>> {
    let hash = hash_token(presented_token);
    let worker = db.find_worker_by_token_hash(&hash)?;
    Ok(worker.filter(|w| w.revoked_at.is_none()))
}
