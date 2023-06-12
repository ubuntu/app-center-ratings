#[tokio::test]
async fn quick_dev() -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get("http://localhost:8080").await?;
    println!("{response:?}");
    Ok(())
}
