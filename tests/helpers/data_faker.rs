use std::fmt::Write;

use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::{Digest, Sha256};

#[allow(dead_code)]
pub fn rnd_sha_256() -> String {
    let data = rnd_string(100);
    let mut hasher = Sha256::new();
    hasher.update(data);

    hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut output, b| {
            // This ignores the error without the overhead of unwrap/expect,
            // This is okay because writing to a string can't fail (barring OOM which won't happen)
            let _ = write!(output, "{b:02x}");
            output
        })
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
