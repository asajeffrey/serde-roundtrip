//! Enable short-circuiting a serialization-then-deserialization roundtrip.

extern crate serde;

use serde::Deserialize;
use serde::Serialize;
use serde::bytes::ByteBuf;
use serde::bytes::Bytes;

use std::borrow::Cow;
use std::borrow::ToOwned;
use std::collections::BinaryHeap;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::collections::VecDeque;
use std::ffi::CStr;
use std::ffi::CString;
use std::hash::BuildHasher;
use std::hash::Hash;
use std::marker::PhantomData;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::SocketAddrV6;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

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

// Types which roundtrip using clone.

macro_rules! roundtrip_via_clone {
    ($t:ty) => {
        impl<T> RoundTrip<T> for $t
            where T: SameDeserialization<SameAs=$t>
        {
            fn round_trip(&self) -> T { T::from(self.clone()) }
        }
        impl SameDeserialization for $t {
            type SameAs = $t;
            fn from(data: $t) -> $t { data }
        }
    };
}

roundtrip_via_clone!(());
roundtrip_via_clone!(ByteBuf);
roundtrip_via_clone!(CString);
roundtrip_via_clone!(Duration);
roundtrip_via_clone!(IpAddr);
roundtrip_via_clone!(Ipv4Addr);
roundtrip_via_clone!(Ipv6Addr);
roundtrip_via_clone!(PathBuf);
roundtrip_via_clone!(SocketAddr);
roundtrip_via_clone!(SocketAddrV4);
roundtrip_via_clone!(SocketAddrV6);
roundtrip_via_clone!(String);
roundtrip_via_clone!(bool);
roundtrip_via_clone!(char);
roundtrip_via_clone!(f32);
roundtrip_via_clone!(f64);
roundtrip_via_clone!(i16);
roundtrip_via_clone!(i32);
roundtrip_via_clone!(i64);
roundtrip_via_clone!(i8);
roundtrip_via_clone!(isize);
roundtrip_via_clone!(u16);
roundtrip_via_clone!(u32);
roundtrip_via_clone!(u64);
roundtrip_via_clone!(u8);
roundtrip_via_clone!(usize);

// Types which roundtrip using to_owned.

macro_rules! roundtrip_via_to_owned {
    ($t:ty) => {
        impl<T> RoundTrip<T> for $t
            where T: SameDeserialization<SameAs=<$t as ToOwned>::Owned>
        {
            fn round_trip(&self) -> T { T::from(self.to_owned()) }
        }
    };
}

roundtrip_via_to_owned!(CStr);
roundtrip_via_to_owned!(Path);
roundtrip_via_to_owned!(str);

// Type constructors which roundtrip by dereferencing to their type argument

macro_rules! roundtrip_via_deref {
    ($F: ident) => {
        impl<S,T> RoundTrip<T> for $F<S> where
            S: RoundTrip<T>,
            T: Deserialize,
        {
            fn round_trip(&self) -> T { T::from(self.deref().round_trip()) }
        }
        impl<T> SameDeserialization for $F<T> where
            T: SameDeserialization,
        {
            type SameAs = T::SameAs;
            fn from(data: T::SameAs) -> $F<T> { $F::new(T::from(data)) }
        }
    }
}

roundtrip_via_deref!(Arc);
roundtrip_via_deref!(Box);
roundtrip_via_deref!(Rc);

// Fixed-size arrays

macro_rules! array_impls {
    ($zero:expr) => {
        impl<S,T,Ts> RoundTrip<Ts> for [S; $zero] where
            S: RoundTrip<T>,
            T: Deserialize,
            Ts: SameDeserialization<SameAs=[T; $zero]>,
        {
            fn round_trip(&self) -> Ts { Ts::from([]) }
        }
    };

    ($len:expr, $($indices:expr),*) => {
        impl<S,T,Ts> RoundTrip<Ts> for [S; $len] where
            S: RoundTrip<T>,
            T: Deserialize,
            Ts: SameDeserialization<SameAs=[T; $len]>,
        {
            fn round_trip(&self) -> Ts { Ts::from([ $(self[$len-($indices+1)].round_trip()),* ]) }
        }
        array_impls!($($indices),*);
    };
}

array_impls!(32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0);

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

impl<'a,T> RoundTrip<T> for Bytes<'a>
    where T: SameDeserialization<SameAs=ByteBuf>
{
    fn round_trip(&self) -> T { T::from(ByteBuf::from(self.to_vec())) }
}

impl<T> SameDeserialization for Vec<T> where
    T: Deserialize,
{
    type SameAs = Vec<T>;
    fn from(data: Vec<T>) -> Vec<T> { data }
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

impl<'a,S:?Sized,T> RoundTrip<T> for Cow<'a,S> where
    S: ToOwned + RoundTrip<T>,
    T: Deserialize,
{
    fn round_trip(&self) -> T { (**self).round_trip() }
}

impl<'a,T:?Sized> SameDeserialization for Cow<'a,T> where
    T: ToOwned,
    T::Owned: SameDeserialization,
{
    type SameAs = <T::Owned as SameDeserialization>::SameAs;
    fn from(data: Self::SameAs) -> Self { Cow::Owned(SameDeserialization::from(data)) }
}

// Tuples

impl <S0, T0, T> RoundTrip<T> for (S0,) where
    S0: RoundTrip<T0>,
    T0: Deserialize,
    T: SameDeserialization<SameAs=(T0,)>,
{
    fn round_trip(&self) -> T { T::from((self.0.round_trip(),)) }
}

impl <T> SameDeserialization for (T,) where
    T: Deserialize,
{
    type SameAs = (T,);
    fn from(data: (T,)) -> (T,) { data }
}

macro_rules! tuple_impls {
    ($($xs:ident : $Ss:ident => $Ts:ident),*) => {
        impl<$($Ss),*,$($Ts),*,T> RoundTrip<T> for ($($Ss),*) where
            $($Ss: RoundTrip<$Ts>),*,
            $($Ts: Deserialize),*,
            T: SameDeserialization<SameAs=($($Ts),*)>,
        {
            fn round_trip(&self) -> T {
                let ($(ref $xs),*) = *self;
                T::from(($($xs.round_trip()),*))
            }
        }
        impl<$($Ts),*> SameDeserialization for ($($Ts),*) where
            $($Ts: Deserialize),*,
        {
            type SameAs = ($($Ts),*);
            fn from(data: ($($Ts),*)) -> ($($Ts),*) { data }
        }
    };
}

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6, x_7: S7 => T7);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6, x_7: S7 => T7,
             x_8: S8 => T8);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6, x_7: S7 => T7,
             x_8: S8 => T8, x_9: S9 => T9);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6, x_7: S7 => T7,
             x_8: S8 => T8, x_9: S9 => T9, x_a: SA => TA);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6, x_7: S7 => T7,
             x_8: S8 => T8, x_9: S9 => T9, x_a: SA => TA, x_b: SB => TB);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6, x_7: S7 => T7,
             x_8: S8 => T8, x_9: S9 => T9, x_a: SA => TA, x_b: SB => TB,
             x_c: SC => TC);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6, x_7: S7 => T7,
             x_8: S8 => T8, x_9: S9 => T9, x_a: SA => TA, x_b: SB => TB,
             x_c: SC => TC, x_d: SD => TD);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6, x_7: S7 => T7,
             x_8: S8 => T8, x_9: S9 => T9, x_a: SA => TA, x_b: SB => TB,
             x_c: SC => TC, x_d: SD => TD, x_e: SE => TE);

tuple_impls!(x_0: S0 => T0, x_1: S1 => T1, x_2: S2 => T2, x_3: S3 => T3,
             x_4: S4 => T4, x_5: S5 => T5, x_6: S6 => T6, x_7: S7 => T7,
             x_8: S8 => T8, x_9: S9 => T9, x_a: SA => TA, x_b: SB => TB,
             x_c: SC => TC, x_d: SD => TD, x_e: SE => TE, x_f: SF => TF);

// Phantom data

impl<S,T> RoundTrip<T> for PhantomData<S> where
    T: SameDeserialization<SameAs=PhantomData<S>>,
{
    fn round_trip(&self) -> T { T::from(PhantomData) }
}

impl<T> SameDeserialization for PhantomData<T> {
    type SameAs = PhantomData<T>;
    fn from(data: PhantomData<T>) -> PhantomData<T> { data }
}

// Options

impl<S0,T0,T> RoundTrip<T> for Option<S0> where
    S0: RoundTrip<T0>,
    T0: Deserialize,
    T: SameDeserialization<SameAs=Option<T0>>,
{
    fn round_trip(&self) -> T { T::from(self.as_ref().map(RoundTrip::round_trip)) }
}

impl<T> SameDeserialization for Option<T> where
    T: Deserialize
{
    type SameAs = Option<T>;
    fn from(data: Option<T>) -> Option<T> { data }
}

// Results

impl<S0,S1,T0,T1,T> RoundTrip<T> for Result<S0,S1> where
    S0: RoundTrip<T0>,
    S1: RoundTrip<T1>,
    T0: Deserialize,
    T1: Deserialize,
    T: SameDeserialization<SameAs=Result<T0,T1>>,
{
    fn round_trip(&self) -> T { T::from(self.as_ref().map(RoundTrip::round_trip).map_err(RoundTrip::round_trip)) }
}

impl<T0,T1> SameDeserialization for Result<T0,T1> where
    T0: Deserialize,
    T1: Deserialize,
{
    type SameAs = Result<T0,T1>;
    fn from(data: Result<T0,T1>) -> Result<T0,T1> { data }
}

// BinaryHeap

impl<S0,T0,T> RoundTrip<T> for BinaryHeap<S0> where
    S0: Ord+RoundTrip<T0>,
    T0: Ord+Deserialize,
    T: SameDeserialization<SameAs=BinaryHeap<T0>>,
{
    fn round_trip(&self) -> T { T::from(self.iter().map(RoundTrip::round_trip).collect()) }
}

impl<T0> SameDeserialization for BinaryHeap<T0> where
    T0: Ord+Deserialize,
{
    type SameAs = BinaryHeap<T0>;
    fn from(data: BinaryHeap<T0>) -> BinaryHeap<T0> { data }
}

// BTreeMap

impl<S0,S1,T0,T1,T> RoundTrip<T> for BTreeMap<S0,S1> where
    S0: Ord+RoundTrip<T0>,
    S1: RoundTrip<T1>,
    T0: Ord+Deserialize,
    T1: Deserialize,
    T: SameDeserialization<SameAs=BTreeMap<T0,T1>>,
{
    fn round_trip(&self) -> T {
        T::from(self.iter().map(|(x1,x2)| (x1.round_trip(), x2.round_trip())).collect())
    }
}

impl<T0,T1> SameDeserialization for BTreeMap<T0,T1> where
    T0: Ord+Deserialize,
    T1: Deserialize,
{
    type SameAs = BTreeMap<T0,T1>;
    fn from(data: BTreeMap<T0,T1>) -> BTreeMap<T0,T1> { data }
}

// BTreeSet

impl<S0,T0,T> RoundTrip<T> for BTreeSet<S0> where
    S0: Ord+RoundTrip<T0>,
    T0: Ord+Deserialize,
    T: SameDeserialization<SameAs=BTreeSet<T0>>,
{
    fn round_trip(&self) -> T { T::from(self.iter().map(RoundTrip::round_trip).collect()) }
}

impl<T0> SameDeserialization for BTreeSet<T0> where
    T0: Ord+Deserialize,
{
    type SameAs = BTreeSet<T0>;
    fn from(data: BTreeSet<T0>) -> BTreeSet<T0> { data }
}

// HashMap

impl<S0,S1,T0,T1,H,T> RoundTrip<T> for HashMap<S0,S1,H> where
    S0: Eq+Hash+RoundTrip<T0>,
    S1: RoundTrip<T1>,
    T0: Eq+Hash+Deserialize,
    T1: Deserialize,
    H: BuildHasher+Default,
    T: SameDeserialization<SameAs=HashMap<T0,T1,H>>,
{
    fn round_trip(&self) -> T {
        T::from(self.iter().map(|(x1,x2)| (x1.round_trip(), x2.round_trip())).collect())
    }
}

impl<T0,T1,H> SameDeserialization for HashMap<T0,T1,H> where
    T0: Eq+Hash+Deserialize,
    T1: Deserialize,
    H: BuildHasher+Default,
{
    type SameAs = HashMap<T0,T1,H>;
    fn from(data: HashMap<T0,T1,H>) -> HashMap<T0,T1,H> { data }
}

// HashSet

impl<S0,T0,H,T> RoundTrip<T> for HashSet<S0,H> where
    S0: Eq+Hash+RoundTrip<T0>,
    T0: Eq+Hash+Deserialize,
    H: BuildHasher+Default,
    T: SameDeserialization<SameAs=HashSet<T0,H>>,
{
    fn round_trip(&self) -> T { T::from(self.iter().map(RoundTrip::round_trip).collect()) }
}

impl<T0,H> SameDeserialization for HashSet<T0,H> where
    T0: Eq+Hash+Deserialize,
    H: BuildHasher+Default,
{
    type SameAs = HashSet<T0,H>;
    fn from(data: HashSet<T0,H>) -> HashSet<T0,H> { data }
}

// LinkedList

impl<S0,T0,T> RoundTrip<T> for LinkedList<S0> where
    S0: RoundTrip<T0>,
    T0: Deserialize,
    T: SameDeserialization<SameAs=LinkedList<T0>>,
{
    fn round_trip(&self) -> T { T::from(self.iter().map(RoundTrip::round_trip).collect()) }
}

impl<T0> SameDeserialization for LinkedList<T0> where
    T0: Deserialize,
{
    type SameAs = LinkedList<T0>;
    fn from(data: LinkedList<T0>) -> LinkedList<T0> { data }
}

// VecDeque

impl<S0,T0,T> RoundTrip<T> for VecDeque<S0> where
    S0: RoundTrip<T0>,
    T0: Deserialize,
    T: SameDeserialization<SameAs=VecDeque<T0>>,
{
    fn round_trip(&self) -> T { T::from(self.iter().map(RoundTrip::round_trip).collect()) }
}

impl<T0> SameDeserialization for VecDeque<T0> where
    T0: Deserialize,
{
    type SameAs = VecDeque<T0>;
    fn from(data: VecDeque<T0>) -> VecDeque<T0> { data }
}

