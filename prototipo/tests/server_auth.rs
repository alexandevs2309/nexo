use basilisco::servidor::auth;

#[test]
fn password_hash_roundtrip() {
    let h = auth::hash_password("correct horse battery staple").unwrap();
    assert!(auth::verify_password("correct horse battery staple", &h));
    assert!(!auth::verify_password("wrong", &h));
}

#[test]
fn token_hash_determinista() {
    let a = auth::token_hash("abc");
    let b = auth::token_hash("abc");
    assert_eq!(a, b);
    assert_ne!(a, auth::token_hash("def"));
}
