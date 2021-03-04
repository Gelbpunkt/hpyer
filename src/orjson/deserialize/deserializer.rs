// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::orjson::{deserialize::cache::*, typeref::*, unicode::*};
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use smallvec::SmallVec;
use std::{borrow::Cow, fmt, ptr::NonNull};
use wyhash::wyhash;

pub fn deserialize(data: &[u8]) -> std::result::Result<NonNull<pyo3::ffi::PyObject>, String> {
    let mut deserializer = serde_json::Deserializer::from_slice(data);

    let seed = JsonValue {};
    match seed.deserialize(&mut deserializer) {
        Ok(obj) => {
            deserializer.end().map_err(|e| e.to_string())?;
            Ok(obj)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[derive(Clone, Copy)]
struct JsonValue;

impl<'de> DeserializeSeed<'de> for JsonValue {
    type Value = NonNull<pyo3::ffi::PyObject>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for JsonValue {
    type Value = NonNull<pyo3::ffi::PyObject>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("JSON")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        ffi!(Py_INCREF(NONE));
        Ok(nonnull!(NONE))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value {
            ffi!(Py_INCREF(TRUE));
            Ok(nonnull!(TRUE))
        } else {
            ffi!(Py_INCREF(FALSE));
            Ok(nonnull!(FALSE))
        }
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyLong_FromLongLong(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyLong_FromUnsignedLongLong(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyFloat_FromDouble(value))))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_str(value.as_str())))
    }

    fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_str(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_str(value)))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        match seq.next_element_seed(self) {
            Ok(None) => Ok(nonnull!(ffi!(PyList_New(0)))),
            Ok(Some(elem)) => {
                let mut elements: SmallVec<[*mut pyo3::ffi::PyObject; 8]> =
                    SmallVec::with_capacity(8);
                elements.push(elem.as_ptr());
                while let Some(elem) = seq.next_element_seed(self)? {
                    elements.push(elem.as_ptr());
                }
                let ptr = ffi!(PyList_New(elements.len() as pyo3::ffi::Py_ssize_t));
                for (i, &obj) in elements.iter().enumerate() {
                    ffi!(PyList_SET_ITEM(ptr, i as pyo3::ffi::Py_ssize_t, obj));
                }
                Ok(nonnull!(ptr))
            }
            Err(err) => std::result::Result::Err(err),
        }
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let dict_ptr = ffi!(PyDict_New());
        while let Some(key) = map.next_key::<Cow<str>>()? {
            let pykey: *mut pyo3::ffi::PyObject;
            let pyhash: pyo3::ffi::Py_hash_t;
            let value = map.next_value_seed(self)?;
            if likely!(key.len() <= 64) {
                let hash = unsafe { wyhash(key.as_bytes(), HASH_SEED) };
                {
                    let map = unsafe {
                        KEY_MAP
                            .get_mut()
                            .unwrap_or_else(|| unsafe { std::hint::unreachable_unchecked() })
                    };
                    let entry = map.entry(&hash).or_insert_with(
                        || hash,
                        || {
                            let pyob = unicode_from_str(&key);
                            hash_str(pyob);
                            CachedKey::new(pyob)
                        },
                    );
                    pykey = entry.get();
                    pyhash = unsafe { (*pykey.cast::<PyASCIIObject>()).hash }
                }
            } else {
                pykey = unicode_from_str(&key);
                pyhash = hash_str(pykey);
            }
            println!(
                "before panic: {:?} {:?} {:?} {:?} {:?}",
                key, dict_ptr, pykey, value, pyhash
            );
            println!("{} {} {}", dict_ptr.is_null(), pykey.is_null(), value.as_ptr().is_null());
            let thing = ffi!(_PyDict_SetItem_KnownHash(
                dict_ptr,
                pykey,
                value.as_ptr(),
                pyhash
            ));
            println!("after panic? {}", thing);
            // counter Py_INCREF in insertdict
            ffi!(Py_DECREF(pykey));
            ffi!(Py_DECREF(value.as_ptr()));
        }
        println!("ok");
        Ok(nonnull!(dict_ptr))
    }
}