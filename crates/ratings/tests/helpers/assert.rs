use ratings::app::interfaces::authentication::jwt::JwtVerifier;

#[allow(dead_code)]
pub fn assert_token_is_valid(value: &str) {
    let jwt = JwtVerifier::from_env();
    assert!(
        jwt.unwrap().decode(value).is_ok(),
        "value should be a valid jwt"
    );
}

#[allow(dead_code)]
pub fn assert_token_is_not_valid(value: &str) {
    let jwt = JwtVerifier::from_env();
    assert!(jwt.unwrap().decode(value).is_err(), "expected invalid jwt");
}
