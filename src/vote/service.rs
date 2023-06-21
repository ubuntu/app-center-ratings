use std::error::Error;

pub async fn process_vote() -> Result<String, Box<dyn Error>> {
    let id = String::from("id-1");

    Ok(id)
}
