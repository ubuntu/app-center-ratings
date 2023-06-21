use common::{get_server_url, setup_integration_test, teardown_integration_test};

mod common;

#[tokio::test]
async fn health_check() -> Result<(), Box<dyn std::error::Error>> {
    setup_integration_test().await?;

    let host = get_server_url();
    let url = format!("{host}/");
    let response = reqwest::get(url).await?;

    let actual = response.text().await.unwrap();
    let expected = "OK";
    assert_eq!(actual, expected);

    teardown_integration_test().await?;
    Ok(())
}
