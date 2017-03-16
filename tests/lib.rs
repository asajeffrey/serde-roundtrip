extern crate serde;
extern crate serde_json;
extern crate serde_roundtrip;

use serde_json::{to_string, from_str};
use serde_roundtrip::RoundTrip;

type Source = &'static [&'static str];
type Target = Vec<String>;
const SOURCE: Source = &["hello","world"];

#[test]
fn test_round_trip() {
    let via_json: Target = from_str(&*to_string(SOURCE).unwrap()).unwrap();
    let via_round_trip: Target = SOURCE.round_trip();
    assert_eq!(via_json, via_round_trip);
}
