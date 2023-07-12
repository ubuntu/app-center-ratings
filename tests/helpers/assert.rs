use ratings::utils::jwt::Jwt;

pub fn assert_token_is_valid(value: &str) {
    let jwt = Jwt::new();
    jwt.decode(value).expect("value should be a valid jwt");
}

pub fn assert_token_is_not_valid(value: &str) {
    let jwt = Jwt::new();
    match jwt.decode(value) {
        Ok(_) => {
            panic!("expected invalid jwt")
        }
        Err(_) => {}
    }
}
