# serde_roundtrip

A trait for when serde supports serializing at one type and deserializing as another.

The main trait is `S: RoundTrip<T>` which means that data of type `S` can be serialized
using serde, then safely deserialized at type `T`. This allows serialization to be safe
without taking ownership of the data, for example serializing a `&[&str]` and deserializing
a `Vec<String>`.

The `RoundTrip<T>` trait provides a method `fn round_trip(&self) -> T`, which has the same
semantics as serializing then deserializing. This allows serialization to be short-circuited
in the case that an in-memory representation can be used.

The `RoundTrip` trait is implemented for the types for which serde provides a serialization.
For user-defined types, the `serde_roundtrip_derive` crate provides a `derive(RoundTrip)`.

For example:
```rust
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
```
