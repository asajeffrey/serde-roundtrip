extern crate serde;
extern crate serde_json;
extern crate serde_roundtrip;

use serde_json::{to_string, from_str};
use serde_roundtrip::RoundTrip;

use std::borrow::Cow;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::net::IpAddr;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_round_trip() {
    type Target = (
        (usize,),
        Vec<IpAddr>,
        Vec<String>,
        Box<Rc<Arc<bool>>>,
        Duration,
        HashMap<String, usize>,
        Cow<'static, str>,
    );
    let source = (
        (37,),
        vec![IpAddr::from_str("127.0.0.1").unwrap(), IpAddr::from_str("2001:0db8:85a3:0000:0000:8a2e:0370:7334").unwrap()],
        &["hello","world"][..],
        true,
        Arc::new(Rc::new(Box::new(Duration::new(1000,0)))),
        HashMap::from_iter(vec![ ("a",1) ]),
        Cow::Borrowed("x"),
    );

    let via_json: Target = from_str(&*to_string(&source).unwrap()).unwrap();
    let via_round_trip: Target = source.round_trip();
    assert_eq!(via_json, via_round_trip);
}
