// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::orjson::{
    exc::*,
    ffi::{PyDict_GET_SIZE, PyTypeObject},
    serialize::{
        dataclass::*, datetime::*, default::*, dict::*, int::*, list::*, str::*, tuple::*, uuid::*,
    },
    typeref::*,
};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use std::ptr::NonNull;

pub const RECURSION_LIMIT: u8 = 255;

pub fn serialize(
    ptr: *mut pyo3::ffi::PyObject,
    default: Option<NonNull<pyo3::ffi::PyObject>>,
) -> Result<String, String> {
    let obtype = pyobject_to_obtype(ptr);
    let obj = PyObjectSerializer::with_obtype(ptr, obtype, 0, 0, default);
    let res = serde_json::to_string(&obj);
    match res {
        Ok(val) => Ok(val),
        Err(err) => Err(err.to_string()),
    }
}

#[derive(Copy, Clone)]
pub enum ObType {
    Str,
    Int,
    Bool,
    None,
    Float,
    List,
    Dict,
    Datetime,
    Date,
    Time,
    Tuple,
    Uuid,
    Dataclass,
    Enum,
    StrSubclass,
    Unknown,
}

#[inline]
pub fn pyobject_to_obtype(obj: *mut pyo3::ffi::PyObject) -> ObType {
    unsafe {
        let ob_type = ob_type!(obj);
        if ob_type == STR_TYPE {
            ObType::Str
        } else if ob_type == INT_TYPE {
            ObType::Int
        } else if ob_type == BOOL_TYPE {
            ObType::Bool
        } else if ob_type == NONE_TYPE {
            ObType::None
        } else if ob_type == FLOAT_TYPE {
            ObType::Float
        } else if ob_type == LIST_TYPE {
            ObType::List
        } else if ob_type == DICT_TYPE {
            ObType::Dict
        } else if ob_type == DATETIME_TYPE {
            ObType::Datetime
        } else {
            pyobject_to_obtype_unlikely(obj)
        }
    }
}

macro_rules! is_subclass {
    ($ob_type:expr, $flag:ident) => {
        unsafe { (((*$ob_type).tp_flags & pyo3::ffi::$flag) != 0) }
    };
}

#[inline(never)]
pub fn pyobject_to_obtype_unlikely(obj: *mut pyo3::ffi::PyObject) -> ObType {
    unsafe {
        let ob_type = ob_type!(obj);
        if ob_type == DATE_TYPE {
            ObType::Date
        } else if ob_type == TIME_TYPE {
            ObType::Time
        } else if ob_type == TUPLE_TYPE {
            ObType::Tuple
        } else if ob_type == UUID_TYPE {
            ObType::Uuid
        } else if (*(ob_type as *mut PyTypeObject)).ob_type == ENUM_TYPE {
            ObType::Enum
        } else if is_subclass!(ob_type, Py_TPFLAGS_UNICODE_SUBCLASS) {
            ObType::StrSubclass
        } else if is_subclass!(ob_type, Py_TPFLAGS_LONG_SUBCLASS) {
            ObType::Int
        } else if is_subclass!(ob_type, Py_TPFLAGS_LIST_SUBCLASS) {
            ObType::List
        } else if is_subclass!(ob_type, Py_TPFLAGS_DICT_SUBCLASS) {
            ObType::Dict
        } else if ffi!(PyDict_Contains((*ob_type).tp_dict, DATACLASS_FIELDS_STR)) == 1 {
            ObType::Dataclass
        } else {
            ObType::Unknown
        }
    }
}

pub struct PyObjectSerializer {
    ptr: *mut pyo3::ffi::PyObject,
    obtype: ObType,
    default_calls: u8,
    recursion: u8,
    default: Option<NonNull<pyo3::ffi::PyObject>>,
}

impl PyObjectSerializer {
    #[inline]
    pub fn new(
        ptr: *mut pyo3::ffi::PyObject,
        default_calls: u8,
        recursion: u8,
        default: Option<NonNull<pyo3::ffi::PyObject>>,
    ) -> Self {
        PyObjectSerializer {
            ptr: ptr,
            obtype: pyobject_to_obtype(ptr),
            default_calls: default_calls,
            recursion: recursion,
            default: default,
        }
    }

    #[inline]
    pub fn with_obtype(
        ptr: *mut pyo3::ffi::PyObject,
        obtype: ObType,
        default_calls: u8,
        recursion: u8,
        default: Option<NonNull<pyo3::ffi::PyObject>>,
    ) -> Self {
        PyObjectSerializer {
            ptr: ptr,
            obtype: obtype,
            default_calls: default_calls,
            recursion: recursion,
            default: default,
        }
    }
}

impl<'p> Serialize for PyObjectSerializer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.obtype {
            ObType::Str => StrSerializer::new(self.ptr).serialize(serializer),
            ObType::StrSubclass => StrSubclassSerializer::new(self.ptr).serialize(serializer),
            ObType::Int => IntSerializer::new(self.ptr).serialize(serializer),
            ObType::None => serializer.serialize_unit(),
            ObType::Float => serializer.serialize_f64(ffi!(PyFloat_AS_DOUBLE(self.ptr))),
            ObType::Bool => serializer.serialize_bool(unsafe { self.ptr == TRUE }),
            ObType::Datetime => DateTime::new(self.ptr).serialize(serializer),
            ObType::Date => Date::new(self.ptr).serialize(serializer),
            ObType::Time => match Time::new(self.ptr) {
                Ok(val) => val.serialize(serializer),
                Err(TimeError::HasTimezone) => err!(TIME_HAS_TZINFO),
            },
            ObType::Uuid => Uuid::new(self.ptr).serialize(serializer),
            ObType::Dict => {
                if unlikely!(self.recursion == RECURSION_LIMIT) {
                    err!(RECURSION_LIMIT_REACHED)
                }
                let len = unsafe { PyDict_GET_SIZE(self.ptr) as usize };
                if unlikely!(len == 0) {
                    serializer.serialize_map(Some(0)).unwrap().end()
                } else {
                    Dict::new(
                        self.ptr,
                        self.default_calls,
                        self.recursion,
                        self.default,
                        len,
                    )
                    .serialize(serializer)
                }
            }
            ObType::List => {
                if unlikely!(self.recursion == RECURSION_LIMIT) {
                    err!(RECURSION_LIMIT_REACHED)
                }
                let len = ffi!(PyList_GET_SIZE(self.ptr)) as usize;
                if unlikely!(len == 0) {
                    serializer.serialize_seq(Some(0)).unwrap().end()
                } else {
                    ListSerializer::new(
                        self.ptr,
                        self.default_calls,
                        self.recursion,
                        self.default,
                        len,
                    )
                    .serialize(serializer)
                }
            }
            ObType::Tuple => {
                TupleSerializer::new(self.ptr, self.default_calls, self.recursion, self.default)
                    .serialize(serializer)
            }
            ObType::Dataclass => {
                if unlikely!(self.recursion == RECURSION_LIMIT) {
                    err!(RECURSION_LIMIT_REACHED)
                }
                let dict = ffi!(PyObject_GetAttr(self.ptr, DICT_STR));
                if likely!(!dict.is_null()) {
                    ffi!(Py_DECREF(dict));
                    DataclassFastSerializer::new(
                        dict,
                        self.default_calls,
                        self.recursion,
                        self.default,
                    )
                    .serialize(serializer)
                } else {
                    unsafe { pyo3::ffi::PyErr_Clear() };
                    DataclassFallbackSerializer::new(
                        self.ptr,
                        self.default_calls,
                        self.recursion,
                        self.default,
                    )
                    .serialize(serializer)
                }
            }
            ObType::Enum => {
                let value = ffi!(PyObject_GetAttr(self.ptr, VALUE_STR));
                ffi!(Py_DECREF(value));
                PyObjectSerializer::new(value, self.default_calls, self.recursion, self.default)
                    .serialize(serializer)
            }
            ObType::Unknown => {
                DefaultSerializer::new(self.ptr, self.default_calls, self.recursion, self.default)
                    .serialize(serializer)
            }
        }
    }
}
