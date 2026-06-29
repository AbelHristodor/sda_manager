use hymnal_core::update::is_transient_failure;

#[test]
fn rate_limit_403_is_transient() {
    // The exact shape self_update/ureq produces on GitHub's unauthenticated
    // rate limit, plus common phrasings.
    assert!(is_transient_failure("UreqError: http status: 403"));
    assert!(is_transient_failure("http status: 429"));
    assert!(is_transient_failure("API rate limit exceeded for 1.2.3.4"));
}

#[test]
fn network_errors_are_transient() {
    assert!(is_transient_failure("dns error: failed to lookup address"));
    assert!(is_transient_failure("Network is unreachable (os error 51)"));
    assert!(is_transient_failure("connection timed out"));
}

#[test]
fn real_failures_are_not_transient() {
    // A genuine problem we'd want surfaced, not silently softened.
    assert!(!is_transient_failure(
        "Could not find the required path in the archive: hymnal-gui"
    ));
    assert!(!is_transient_failure("http status: 404"));
    assert!(!is_transient_failure("permission denied (os error 13)"));
}
