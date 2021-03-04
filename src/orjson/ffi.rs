// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use pyo3::ffi::*;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PyTypeObject {
    pub ob_refcnt: pyo3::ffi::Py_ssize_t,
    pub ob_type: *mut pyo3::ffi::PyTypeObject,
    pub ma_used: pyo3::ffi::Py_ssize_t,
    pub tp_name: *const c_char,
    // ...
}

#[allow(non_snake_case)]
#[inline(always)]
pub unsafe fn PyDict_GET_SIZE(op: *mut PyObject) -> Py_ssize_t {
    (*op.cast::<PyDictObject>()).ma_used
}
