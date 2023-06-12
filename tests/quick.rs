#[tokio::test]
async fn quick_dev() -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get("http://localhost:18080").await?;
    let text = response.text().await.unwrap();

    assert_eq!(text, "OK");

    Ok(())
}
