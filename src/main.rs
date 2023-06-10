use mongodb::{options::ClientOptions, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    // Parse a connection string into an options struct.
    let mut client_options = ClientOptions::parse("mongodb://root:covfefe@localhost:27017").await?;

    // Manually set an option.
    client_options.app_name = Some("My App".to_string());

    // Get a handle to the deployment.
    let client = Client::with_options(client_options)?;

    // List the names of the databases in that deployment.
    for db_name in client.list_database_names(None, None).await? {
        println!("{}", db_name);
    }

    Ok(())
}
