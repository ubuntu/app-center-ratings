mod utils;

use utils::lifecycle::{after, before};

#[tokio::test]
async fn quick() {
    before().await;

    let url = format!("{}/", utils::server_url());
    let response = reqwest::get(url).await.unwrap();

    println!("response: {response:?}");

    let actual = response.text().await.unwrap();
    let expected = "id-1";

    println!("body: {actual}");
    assert_eq!(actual, expected);

    after().await;
}
