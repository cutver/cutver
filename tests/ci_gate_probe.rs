//! Temporary probe for #257: proves the required check blocks a red pull request.
//! This file is deleted immediately after the demonstration.

#[test]
fn this_test_fails_on_purpose() {
    assert!(false, "deliberate failure to prove the CI gate blocks the merge");
}
