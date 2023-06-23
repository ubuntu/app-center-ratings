mod utils;

use utils::{
    lifecycle::{after, before},
    server_url,
};

#[tokio::test]
async fn health_check() {
    before().await;

    let url = server_url();
    let url = format!("{url}/");
    let response = reqwest::get(url).await.unwrap();

    let actual = response.text().await.unwrap();
    let expected = "OK";

    assert_eq!(actual, expected);

    after().await;
}
