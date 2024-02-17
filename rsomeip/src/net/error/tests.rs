use super::*;

#[test]
fn error_from_str() {
    let error = Error::Failure("bug");
    assert_eq!(
        error.to_string(),
        String::from("operation failed unexpectedly: bug")
    );
}
