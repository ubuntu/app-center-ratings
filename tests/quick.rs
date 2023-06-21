#[tokio::test]
async fn quick_dev() -> Result<(), Box<dyn std::error::Error>> {
    let url = "http://localhost:18080/v1/vote";
    let response = reqwest::get(url).await?;

    println!("response: {response:?}");

    let actual = response.text().await.unwrap();
    let expected = "id-1";

    println!("body: {actual}");
    assert_eq!(actual, expected);

    Ok(())
}
