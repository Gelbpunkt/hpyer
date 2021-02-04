use pyo3::{prelude::pymodule, types::PyModule, PyResult, Python};

mod asyncio;
mod client;
mod error;
mod runtime;

#[pymodule]
fn hpyer(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<client::ClientSession>()?;
    m.add_class::<client::ClientResponse>()?;
    m.add("Error", py.get_type::<error::Error>())?;

    Ok(())
}
