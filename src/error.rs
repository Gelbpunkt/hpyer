use pyo3::{create_exception, exceptions::PyException};

create_exception!(zangy, Error, PyException);
