use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::{Digest, Sha256};

#[allow(dead_code)]
pub fn rnd_sha_256() -> String {
    let data = rnd_string(100);
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let result: String = result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    result
}

pub fn rnd_id() -> String {
    rnd_string(32)
}

fn rnd_string(len: usize) -> String {
    let rng = rand::thread_rng();
    rng.sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
