use super::service::process_vote;



pub async fn vote() -> String {
    let result = process_vote().await.unwrap();
    result
}
