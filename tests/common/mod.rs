use urate::utils::env;

pub async fn setup_integration_test() -> Result<(), Box<dyn std::error::Error>> {
    env::init();

    // Questions
    // TODO: What is the idiomatic approach to setting up / tearing down for ITs?
    // TODO: What is the approach to clearing state?

    Ok(())
}

pub async fn teardown_integration_test() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn get_server_url() -> String {
    let address = env::construct_address_and_port();
    format!("http://{address}")
}
