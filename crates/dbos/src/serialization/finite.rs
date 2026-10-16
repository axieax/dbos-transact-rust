//! Refusing non-finite floats on the way into JSON.
//!
//! `serde_json` writes `NaN`, `inf` and `-inf` as `null` without an error, so the value reads back
//! as something else: an `f64` field fails to decode and an `Option<f64>` becomes `None`. Checking
//! the encoded text afterwards can't catch it, since a `null` there may be a real `None`. So the
//! check happens on the way in: [`Finite`] serializes a value through a serializer that passes
//! every call on to the real one, except that it refuses a non-finite `f32` or `f64`, wherever in
//! the value it is.

use std::fmt::Display;

use serde::ser::{self, Serialize, Serializer};

/// `value`, refusing any non-finite float in it when serialized.
pub(super) struct Finite<'a, T: ?Sized>(pub(super) &'a T);

impl<T: Serialize + ?Sized> Serialize for Finite<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(Checked(serializer))
    }
}

/// A serializer, or one of its compound serializers, whose floats are checked and whose nested
/// values are wrapped in [`Finite`] in turn.
struct Checked<S>(S);

fn refuse<E: ser::Error>(value: impl Display) -> E {
    E::custom(format_args!(
        "{value} is not a finite number, and JSON has no way to store one"
    ))
}

macro_rules! pass {
    ($($method:ident($ty:ty)),* $(,)?) => {
        $(fn $method(self, v: $ty) -> Result<S::Ok, S::Error> {
            self.0.$method(v)
        })*
    };
}

impl<S: Serializer> Serializer for Checked<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = Checked<S::SerializeSeq>;
    type SerializeTuple = Checked<S::SerializeTuple>;
    type SerializeTupleStruct = Checked<S::SerializeTupleStruct>;
    type SerializeTupleVariant = Checked<S::SerializeTupleVariant>;
    type SerializeMap = Checked<S::SerializeMap>;
    type SerializeStruct = Checked<S::SerializeStruct>;
    type SerializeStructVariant = Checked<S::SerializeStructVariant>;

    pass!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
        serialize_char(char),
        serialize_str(&str),
        serialize_bytes(&[u8]),
        serialize_unit_struct(&'static str),
    );

    fn serialize_f32(self, v: f32) -> Result<S::Ok, S::Error> {
        if !v.is_finite() {
            return Err(refuse(v));
        }
        self.0.serialize_f32(v)
    }

    fn serialize_f64(self, v: f64) -> Result<S::Ok, S::Error> {
        if !v.is_finite() {
            return Err(refuse(v));
        }
        self.0.serialize_f64(v)
    }

    fn serialize_none(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_none()
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<S::Ok, S::Error> {
        self.0.serialize_some(&Finite(value))
    }

    fn serialize_unit(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
    ) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit_variant(name, index, variant)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<S::Ok, S::Error> {
        self.0.serialize_newtype_struct(name, &Finite(value))
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<S::Ok, S::Error> {
        self.0
            .serialize_newtype_variant(name, index, variant, &Finite(value))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, S::Error> {
        self.0.serialize_seq(len).map(Checked)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, S::Error> {
        self.0.serialize_tuple(len).map(Checked)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, S::Error> {
        self.0.serialize_tuple_struct(name, len).map(Checked)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, S::Error> {
        self.0
            .serialize_tuple_variant(name, index, variant, len)
            .map(Checked)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, S::Error> {
        self.0.serialize_map(len).map(Checked)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, S::Error> {
        self.0.serialize_struct(name, len).map(Checked)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, S::Error> {
        self.0
            .serialize_struct_variant(name, index, variant, len)
            .map(Checked)
    }

    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }
}

impl<S: ser::SerializeSeq> ser::SerializeSeq for Checked<S> {
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), S::Error> {
        self.0.serialize_element(&Finite(value))
    }

    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}

impl<S: ser::SerializeTuple> ser::SerializeTuple for Checked<S> {
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), S::Error> {
        self.0.serialize_element(&Finite(value))
    }

    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}

impl<S: ser::SerializeTupleStruct> ser::SerializeTupleStruct for Checked<S> {
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), S::Error> {
        self.0.serialize_field(&Finite(value))
    }

    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}

impl<S: ser::SerializeTupleVariant> ser::SerializeTupleVariant for Checked<S> {
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), S::Error> {
        self.0.serialize_field(&Finite(value))
    }

    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}

impl<S: ser::SerializeMap> ser::SerializeMap for Checked<S> {
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), S::Error> {
        self.0.serialize_key(&Finite(key))
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), S::Error> {
        self.0.serialize_value(&Finite(value))
    }

    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}

impl<S: ser::SerializeStruct> ser::SerializeStruct for Checked<S> {
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), S::Error> {
        self.0.serialize_field(key, &Finite(value))
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), S::Error> {
        self.0.skip_field(key)
    }

    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}

impl<S: ser::SerializeStructVariant> ser::SerializeStructVariant for Checked<S> {
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), S::Error> {
        self.0.serialize_field(key, &Finite(value))
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), S::Error> {
        self.0.skip_field(key)
    }

    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}
