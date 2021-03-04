// SPDX-License-Identifier: (Apache-2.0 OR MIT)

#![allow(unused_unsafe)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::zero_prefixed_literal)]

#[macro_use]
mod util;

mod deserialize;
mod exc;
mod ffi;
mod opt;
mod serialize;
pub mod typeref;
mod unicode;

use pyo3::{exceptions::PyValueError, ffi::*, PyResult};

pub fn loads(obj: &[u8]) -> PyResult<*mut PyObject> {
    match deserialize::deserialize(obj) {
        Ok(val) => Ok(val.as_ptr()),
        Err(err) => Err(PyValueError::new_err(err)),
    }
}

pub fn dumps(args: *mut PyObject) -> PyResult<String> {
    match serialize::serialize(args, None, 0 as opt::Opt) {
        Ok(val) => Ok(val),
        Err(err) => Err(PyValueError::new_err(err)),
    }
}
