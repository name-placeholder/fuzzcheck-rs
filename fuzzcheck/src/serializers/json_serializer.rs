extern crate decent_serde_json_alternative;
extern crate json;

// #[cfg(feature = "fuzzcheck_traits_through_fuzzcheck")]
// use fuzzcheck::fuzzcheck_traits;

use std::marker::PhantomData;

/// `JsonSerializer<T>` uses `json` and `serde_json_alternative` to serialize the test
/// inputs (of arbitrary type `T: FromJson + ToJson`) to a json file.
pub struct JsonSerializer<S> {
    phantom: PhantomData<S>,
}

impl<S> Default for JsonSerializer<S> {
    #[no_coverage]
    fn default() -> Self {
        Self { phantom: PhantomData }
    }
}

impl<S> fuzzcheck_traits::Serializer for JsonSerializer<S>
where
    S: decent_serde_json_alternative::FromJson + decent_serde_json_alternative::ToJson,
{
    type Value = S;

    #[no_coverage]
    fn is_utf8(&self) -> bool {
        true
    }
    #[no_coverage]
    fn extension(&self) -> &str {
        "json"
    }
    #[no_coverage]
    fn from_data(&self, data: &[u8]) -> Option<S> {
        let s = String::from_utf8_lossy(data);
        let j = json::parse(&s).ok()?;
        S::from_json(&j)
    }
    #[no_coverage]
    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        let j = value.to_json();
        let mut res = vec![];
        j.write_pretty(&mut res, 4).unwrap();
        res
    }
}