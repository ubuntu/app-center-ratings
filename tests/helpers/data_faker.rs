use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::{Digest, Sha256};

pub fn rnd_sha_256() -> String {
    let mut rng = rand::thread_rng();
    let data: String = rng
        .sample_iter(&Alphanumeric)
        .take(100)
        .map(char::from)
        .collect();
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let result: String = result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    result
}
