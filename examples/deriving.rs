extern crate serde;
extern crate serde_json;
extern crate serde_roundtrip;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_roundtrip_derive;

use serde_roundtrip::RoundTrip;

// A type which can be round-tripped by serde.
// The type might be changed by round-tripping,
// for example a message might be sent as a `Msg<&str>`
// and arrive as a `Msg<String>`.
#[derive(Serialize, Deserialize, RoundTrip, Debug, PartialEq, Eq)]
struct Msg<T>(T);

fn main() {
    // Create a message at type `Msg<&str>`
    let msg: Msg<&'static str> = Msg("hello");

    // Round-trip it via JSON.
    let json = serde_json::to_string(&msg).unwrap();
    let round_tripped: Msg<String> = serde_json::from_str(&*json).unwrap();

    // This is the same as calling the `round_trip()` method.
    assert_eq!(round_tripped, msg.round_trip());
}
