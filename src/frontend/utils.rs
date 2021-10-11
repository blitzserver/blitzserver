use serde::de::Error;

pub fn from_str<'de, D, S>(deserializer: D) -> Result<S, D::Error>
where
    D: serde::Deserializer<'de>,
    S: std::str::FromStr,
{
    let s = <&str as serde::Deserialize>::deserialize(deserializer)?;
    S::from_str(&s).map_err(|_| D::Error::custom("could not parse string"))
}
pub fn from_str_seq<'de, D, S>(deserializer: D) -> Result<Vec<S>, D::Error>
where
    D: serde::Deserializer<'de>,
    S: std::str::FromStr,
{
    let s = <Vec<&str> as serde::Deserialize>::deserialize(deserializer)?;
    s.iter()
        .map(|s| S::from_str(&s).map_err(|_| D::Error::custom("could not parse string")))
        .collect()
}

pub fn de_map_to_scalar<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    // define a visitor that deserializes
    // `ActualData` encoded as json within a string
    struct MapVisitor;

    impl<'de> serde::de::Visitor<'de> for MapVisitor {
        type Value = i32;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("A length 1 sequence containing an int")
        }
        fn visit_seq<A>(self, mut v: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut last = Err(A::Error::custom("Zero length seq"));
            while let Some(v) = v.next_element::<String>()? {
                last = Ok(v.parse().unwrap())
            }
            last
        }
    }
    // use our visitor to deserialize an `ActualValue`
    deserializer.deserialize_seq(MapVisitor)
}
pub fn default_one() -> i32 {
    1
}
pub fn default_five() -> i32 {
    5
}
pub fn default_two() -> i32 {
    2
}
pub fn default_ten() -> i32 {
    10
}
pub fn default_forty() -> i32 {
    40
}
pub fn default_hundred() -> i32 {
    100
}
