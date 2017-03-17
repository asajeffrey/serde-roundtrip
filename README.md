# serde_roundtrip

A trait for when serde supports serializing at one type and deserializing as another.

The main trait is `S: RoundTrip<T>` which means that data of type `S` can be serialized
using serde, then safely deserialized at type `T`. This allows serialization to be safe
without taking ownership of the data, for example serializing a `&[&str]` and deserializing
a `Vec<String>`.

The `RoundTrip<T>` provides a method `fn round_trip(&self) -> T`, which has the same
semantics as serializing then deserializing. This allows serialization to be short-circuited
in the case that an in-memory representation can be used.

The `RoundTrip` trait is implemented for the types for which serde provides a serialization.
For user-defined types, the `serde_roundtrip_derive` crate provides a `derive(RoundTrip)`.
