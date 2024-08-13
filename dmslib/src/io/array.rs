use ndarray::{Array2, ArrayView1};
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

/// Private helper for 2D array serialization.
/// Array is serialized as list of lists.
pub struct Array2Serializer<'a, T>(pub &'a Array2<T>);

/// Private helper for 2D array serialization.
/// This is a row in array.
struct ArrayRowSerializer<'a, T>(ArrayView1<'a, T>);

impl<'a, T: Serialize> Serialize for Array2Serializer<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.shape()[0]))?;
        for row in self.0.rows() {
            seq.serialize_element(&ArrayRowSerializer(row))?;
        }
        seq.end()
    }
}

impl<'a, T: Serialize> Serialize for ArrayRowSerializer<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for i in self.0.iter() {
            seq.serialize_element(i)?;
        }
        seq.end()
    }
}
