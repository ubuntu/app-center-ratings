use ratings::utils::env::get_socket;

pub fn get_server_base_url() -> String {
    let socket = get_socket();
    format!("http://{socket}/")
}
