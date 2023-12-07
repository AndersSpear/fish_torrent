#[cfg(test)]
use super::*;

#[test]
fn return_true() {
    true
}

#[test]
#[should_panic]
fn this_should_panic() {
    panic!();
}
