use http::{HeaderMap, Method, StatusCode, Version};
use pyo3::{
    exceptions::PyValueError,
    prelude::{pyclass, pymethods},
    types::PyDict,
    IntoPy, PyObject, PyResult, Python,
};
use reqwest::{Client, Response, Url};

use std::convert::TryFrom;

use crate::{
    asyncio::{create_future, set_fut_exc, set_fut_result},
    error::Error,
    runtime::RUNTIME,
    types::HttpVersion,
};

#[pyclass]
pub struct ClientSession {
    client: Client,
}

#[pymethods]
impl ClientSession {
    // https://github.com/aio-libs/aiohttp/blob/master/aiohttp/client.py#L193
    #[new]
    #[args(kwargs = "**")]
    fn new(_kwargs: Option<&PyDict>) -> Self {
        let client = Client::builder().build().unwrap();

        Self { client }
    }

    // https://github.com/aio-libs/aiohttp/blob/master/aiohttp/client.py#L306
    #[args(kwargs = "**")]
    fn request(&self, method: &str, url: &str, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        let (fut, res_fut, loop_) = create_future()?;

        let method = match Method::try_from(method) {
            Ok(m) => m,
            Err(e) => return Err(PyValueError::new_err(e.to_string())),
        };

        // TODO: Support at least params, data and json
        // TODO: Cookies and headers

        let req = self.client.request(method, url).body("").build().unwrap();
        let client = self.client.clone();

        RUNTIME.spawn(async move {
            let resp = client.execute(req).await;

            match resp {
                Ok(r) => {
                    let gil = Python::acquire_gil();
                    let py = gil.python();
                    let resp = ClientResponse::new(r).into_py(py);
                    let _ = set_fut_result(loop_, fut, resp);
                }
                Err(e) => {
                    let _ = set_fut_exc(loop_, fut, Error::new_err(e.to_string()));
                }
            }
        });

        Ok(res_fut)
    }

    #[args(kwargs = "**")]
    #[inline]
    fn get(&self, url: &str, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request("GET", url, kwargs)
    }

    #[args(kwargs = "**")]
    #[inline]
    fn options(&self, url: &str, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request("OPTIONS", url, kwargs)
    }

    #[args(kwargs = "**")]
    #[inline]
    fn head(&self, url: &str, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request("HEAD", url, kwargs)
    }

    #[args(kwargs = "**")]
    #[inline]
    fn post(&self, url: &str, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request("POST", url, kwargs)
    }

    #[args(kwargs = "**")]
    #[inline]
    fn put(&self, url: &str, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request("PUT", url, kwargs)
    }

    #[args(kwargs = "**")]
    #[inline]
    fn patch(&self, url: &str, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request("PATCH", url, kwargs)
    }

    #[args(kwargs = "**")]
    #[inline]
    fn delete(&self, url: &str, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request("DELETE", url, kwargs)
    }
}

#[pyclass]
pub struct ClientResponse {
    response: Option<Response>,
    status: StatusCode,
    version: Version,
    headers: HeaderMap,
    url: Url,
}

impl ClientResponse {
    fn new(response: Response) -> Self {
        let status = response.status();
        let version = response.version();
        let headers = response.headers().to_owned();
        let url = response.url().to_owned();

        Self {
            response: Some(response),
            status,
            version,
            headers,
            url,
        }
    }
}

#[pymethods]
impl ClientResponse {
    fn read(&mut self) -> PyResult<PyObject> {
        let (fut, res_fut, loop_) = create_future()?;

        let body = match self.response.take() {
            Some(b) => b,
            None => return Err(Error::new_err("Response body has already been read")),
        };

        RUNTIME.spawn(async move {
            let body = body.bytes().await;

            match body {
                Ok(b) => {
                    let gil = Python::acquire_gil();
                    let py = gil.python();
                    let _ = set_fut_result(loop_, fut, b.into_py(py));
                }
                Err(e) => {
                    let _ = set_fut_exc(loop_, fut, Error::new_err(e.to_string()));
                }
            }
        });

        Ok(res_fut)
    }

    fn text(&mut self) -> PyResult<PyObject> {
        let (fut, res_fut, loop_) = create_future()?;

        let body = match self.response.take() {
            Some(b) => b,
            None => return Err(Error::new_err("Response body has already been read")),
        };

        RUNTIME.spawn(async move {
            let text = body.text().await;

            match text {
                Ok(string) => {
                    let gil = Python::acquire_gil();
                    let py = gil.python();
                    let _ = set_fut_result(loop_, fut, string.into_py(py));
                }
                Err(e) => {
                    let _ = set_fut_exc(loop_, fut, Error::new_err(e.to_string()));
                }
            }
        });

        Ok(res_fut)
    }

    #[getter]
    fn status(&self) -> u16 {
        self.status.as_u16()
    }

    #[getter]
    fn version(&self) -> HttpVersion {
        HttpVersion::from(self.version)
    }
}
