mod common;

use common::TestHelper;
use simple_test_case::test_case;

#[test_case("notarealhash"; "short")]
#[test_case("abcdefghijkabcdefghijkabcdefghijkabcdefghijkabcdefghijkabcdefgh"; "one char too short")]
#[test_case("abcdefghijkabcdefghijkabcdefghijkabcdefghijkabcdefghijkabcdefghijk"; "one char too long")]
#[tokio::test]
async fn invalid_client_hashes_are_rejected(bad_hash: &str) -> anyhow::Result<()> {
    let t = TestHelper::new();

    let res = t.authenticate(bad_hash.to_string()).await;
    assert!(res.is_err(), "{res:?}");

    Ok(())
}

#[tokio::test]
async fn valid_client_hashes_can_authenticate_multiple_times() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let client_hash = t.random_sha_256();

    let token1 = t.authenticate(client_hash.clone()).await?;
    let token2 = t.authenticate(client_hash.clone()).await?;

    assert_eq!(token1, token2);
    t.assert_valid_jwt(&token1);
    t.assert_valid_jwt(&token2);

    Ok(())
}

