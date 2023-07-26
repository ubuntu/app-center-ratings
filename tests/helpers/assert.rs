use ratings::utils::jwt::Jwt;

#[allow(dead_code)]
pub fn assert_token_is_valid(value: &str) {
    let jwt = Jwt::new();
    assert!(jwt.decode(value).is_ok(), "value should be a valid jwt");
}

#[allow(dead_code)]
pub fn assert_token_is_not_valid(value: &str) {
    let jwt = Jwt::new();
    assert!(jwt.decode(value).is_err(), "expected invalid jwt");
}
