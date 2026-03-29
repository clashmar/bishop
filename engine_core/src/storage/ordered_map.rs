use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::hash::Hash;

pub fn serialize<S, K, V>(map: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    K: Eq + Hash + Ord + Serialize,
    V: Serialize,
{
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut state = serializer.serialize_map(Some(entries.len()))?;
    for (key, value) in entries {
        state.serialize_entry(key, value)?;
    }
    state.end()
}

pub fn deserialize<'de, D, K, V>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Eq + Hash + Deserialize<'de>,
    V: Deserialize<'de>,
{
    HashMap::<K, V>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct OrderedMapWrapper {
        #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
        values: HashMap<u8, &'static str>,
    }

    #[test]
    fn serialize_emits_entries_in_key_order() {
        let values = HashMap::from([(2, "two"), (1, "one"), (3, "three")]);
        let wrapper = OrderedMapWrapper { values };

        let ron = ron::to_string(&wrapper).expect("serialize ordered map wrapper");

        assert_eq!(ron, "(values:{1:\"one\",2:\"two\",3:\"three\"})");
    }
}
