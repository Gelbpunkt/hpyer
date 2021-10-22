#![feature(core_intrinsics)]
#![allow(unused_unsafe)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::zero_prefixed_literal)]
use pyo3::{prelude::pymodule, types::PyModule, PyResult, Python};

mod asyncio;
mod client;
mod error;
mod runtime;
mod types;

#[pymodule]
fn hpyer(py: Python, m: &PyModule) -> PyResult<()> {
    orjson::typeref::init_typerefs();

    m.add_class::<client::ClientSession>()?;
    m.add_class::<client::ClientResponse>()?;
    m.add_class::<types::HttpVersion>()?;
    m.add("Error", py.get_type::<error::Error>())?;

    Ok(())
}
