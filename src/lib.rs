//! Enable short-circuiting a serialization-then-deserialization roundtrip.

extern crate serde;

use serde::Deserialize;
use serde::Serialize;

use std::rc::Rc;

/// This trait specifies when it's OK to perform a serialize-then-deserialiize round trip
///
/// If `S: RoundTrip<T>` then the serialization format of `S` is compatible
/// with the deserialization format of `T`.
pub trait RoundTrip<Target: Deserialize>: Serialize {
    /// This function specifies the behaviour of a round-trip.
    /// If `S: RoundTrip<T>` then serializing `data:S` and then deserializing
    /// it at type `T` should produce the same result as `Ok(data.round_trip())`.
    fn round_trip(&self) -> Target;
}

/// This is a helper trait used by `RoundTrip` implementations, which specifies
/// that two deserializations are compatible.
/// 
/// If `T: SameDeserialization` then the deserialization format of `T` is compatible
/// with the deserialization format of `T::SameAs`.
pub trait SameDeserialization: Deserialize {
    /// The type that has the same deserialization.
    type SameAs: Deserialize;
    /// This function specifies the behaviour of deserialization.
    /// If `T: SameDeserialization` then deserializing at type `T` should
    /// produce the same result as deserializing at type `T::SameAs`
    /// then calling `T::from`.
    fn from(data: Self::SameAs) -> Self;
}

// Arrays

impl<S,T,Ts> RoundTrip<Ts> for Vec<S> where
    S: RoundTrip<T>,
    T: Deserialize,
    Ts: SameDeserialization<SameAs=Vec<T>>
{
    fn round_trip(&self) -> Ts {
        Ts::from(self.iter().map(RoundTrip::round_trip).collect())
    }
}

impl<S,T,Ts> RoundTrip<Ts> for [S] where
    S: RoundTrip<T>,
    T: Deserialize,
    Ts: SameDeserialization<SameAs=Vec<T>>
{
    fn round_trip(&self) -> Ts {
        Ts::from(self.iter().map(RoundTrip::round_trip).collect())
    }
}

impl<T> SameDeserialization for Vec<T> where
    T: Deserialize,
{
    type SameAs = Vec<T>;
    fn from(data: Vec<T>) -> Vec<T> { data }
}

// Strings

impl<T> RoundTrip<T> for String where
    T: SameDeserialization<SameAs=String>
{
    fn round_trip(&self) -> T {
        T::from(self.to_owned())
    }
}

impl<T> RoundTrip<T> for str where
    T: SameDeserialization<SameAs=String>
{
    fn round_trip(&self) -> T {
        T::from(self.to_owned())
    }
}

impl SameDeserialization for String {
    type SameAs = String;
    fn from(data: String) -> String { data }
}

// Refs

impl<'a,S:?Sized,T> RoundTrip<T> for &'a S where
    S: RoundTrip<T>,
    T: Deserialize,
{
    fn round_trip(&self) -> T { (**self).round_trip() }
}

impl<'a,S:?Sized,T> RoundTrip<T> for &'a mut S where
    S: RoundTrip<T>,
    T: Deserialize,
{
    fn round_trip(&self) -> T { (**self).round_trip() }
}

impl<S,T> RoundTrip<T> for Rc<S> where
    S: RoundTrip<T>,
    T: Deserialize,
{
    fn round_trip(&self) -> T { (**self).round_trip() }
}

impl<T> SameDeserialization for Rc<T> where
    T: SameDeserialization
{
    type SameAs = T::SameAs;
    fn from(data: T::SameAs) -> Rc<T> { Rc::new(T::from(data)) }
}

// Base types

impl<T> RoundTrip<T> for usize where
    T: SameDeserialization<SameAs=usize>
{
    fn round_trip(&self) -> T { T::from(*self) }
}

impl SameDeserialization for usize {
    type SameAs = usize;
    fn from(data: usize) -> usize { data }
}

// Pairs (deriving RoundTrip on structs should work this way)

impl<S1,S2,T1,T2,T> RoundTrip<T> for (S1, S2) where
    T: SameDeserialization<SameAs=(T1,T2)>,
    S1: RoundTrip<T1>,
    S2: RoundTrip<T2>,
    T1: Deserialize,
    T2: Deserialize,
{
    fn round_trip(&self) -> T { T::from((self.0.round_trip(), self.1.round_trip())) }
}

impl<T1,T2> SameDeserialization for (T1, T2) where
    T1: Deserialize,
    T2: Deserialize
{
    type SameAs = (T1, T2);
    fn from(data: (T1, T2)) -> (T1, T2) { data }
}
