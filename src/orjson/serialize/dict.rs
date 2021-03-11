// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::orjson::{exc::*, serialize::serializer::*, typeref::*, unicode::*};
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::ptr::NonNull;

pub struct Dict {
    ptr: *mut pyo3::ffi::PyObject,
    default_calls: u8,
    recursion: u8,
    default: Option<NonNull<pyo3::ffi::PyObject>>,
    len: usize,
}

impl Dict {
    pub fn new(
        ptr: *mut pyo3::ffi::PyObject,
        default_calls: u8,
        recursion: u8,
        default: Option<NonNull<pyo3::ffi::PyObject>>,
        len: usize,
    ) -> Self {
        Dict {
            ptr: ptr,
            default_calls: default_calls,
            recursion: recursion,
            default: default,
            len: len,
        }
    }
}

impl<'p> Serialize for Dict {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None).unwrap();
        let mut pos = 0isize;
        let mut str_size: pyo3::ffi::Py_ssize_t = 0;
        let mut key: *mut pyo3::ffi::PyObject = std::ptr::null_mut();
        let mut value: *mut pyo3::ffi::PyObject = std::ptr::null_mut();
        for _ in 0..=self.len - 1 {
            unsafe {
                pyo3::ffi::_PyDict_Next(
                    self.ptr,
                    &mut pos,
                    &mut key,
                    &mut value,
                    std::ptr::null_mut(),
                )
            };
            if unlikely!(unsafe { ob_type!(key) != STR_TYPE }) {
                err!(KEY_MUST_BE_STR)
            }
            {
                let data = read_utf8_from_str(key, &mut str_size);
                if unlikely!(data.is_null()) {
                    err!(INVALID_STR)
                }
                map.serialize_key(str_from_slice!(data, str_size)).unwrap();
            }

            map.serialize_value(&PyObjectSerializer::new(
                value,
                self.default_calls,
                self.recursion + 1,
                self.default,
            ))?;
        }
        map.end()
    }
}
