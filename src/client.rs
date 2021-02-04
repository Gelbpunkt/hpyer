use http::{response::Parts, Method, Response, Uri};
use hyper::{
    body::{aggregate, Buf},
    client::HttpConnector,
    Body, Client, Request,
};
use hyper_rustls::HttpsConnector;
use pyo3::{
    exceptions::PyValueError,
    prelude::{pyclass, pymethods},
    types::PyDict,
    IntoPy, PyObject, PyResult, Python,
};

use std::{convert::TryFrom, str::from_utf8};

use crate::{
    asyncio::{create_future, set_fut_exc, set_fut_result},
    error::Error,
    runtime::RUNTIME,
};

#[pyclass]
pub struct ClientSession {
    client: Client<HttpsConnector<HttpConnector>>,
}

#[pymethods]
impl ClientSession {
    // https://github.com/aio-libs/aiohttp/blob/master/aiohttp/client.py#L193
    #[new]
    #[args(kwargs = "**")]
    fn new(_kwargs: Option<&PyDict>) -> Self {
        let https = HttpsConnector::with_native_roots();
        let client = Client::builder().build(https);

        Self { client }
    }

    // https://github.com/aio-libs/aiohttp/blob/master/aiohttp/client.py#L306
    #[args(kwargs = "**")]
    fn request(&self, method: &str, url: &str, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        let (fut, res_fut, loop_) = create_future()?;

        let uri = match Uri::try_from(url) {
            Ok(u) => u,
            Err(e) => return Err(PyValueError::new_err(e.to_string())),
        };
        let method = match Method::try_from(method) {
            Ok(m) => m,
            Err(e) => return Err(PyValueError::new_err(e.to_string())),
        };

        // TODO: Support at least params, data and json
        // TODO: Cookies and headers

        let mut req = Request::builder()
            .method(method.clone())
            .uri(uri)
            .body(Body::from(""))
            .unwrap();
        let client = self.client.clone();

        RUNTIME.spawn(async move {
            // TODO: Consider using reqwest because redirection handling is awful
            let mut resp = client.request(req).await;
            let mut is_final = false;

            while !is_final {
                if let Ok(r) = &resp {
                    if r.status().is_redirection() {
                        let mut next_url = r.headers().get("Location");
                        if next_url.is_none() {
                            next_url = r.headers().get("Uri");
                        }
                        // TODO: Don't unwrap
                        let url = next_url.unwrap().to_str().unwrap();
                        req = Request::builder()
                            .method(method.clone())
                            .uri(url)
                            .body(Body::from(""))
                            .unwrap();
                        resp = client.request(req).await;
                    } else {
                        is_final = true;
                    }
                } else {
                    is_final = true;
                }
            }

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
    parts: Parts,
    body: Option<Body>,
}

impl ClientResponse {
    fn new(response: Response<Body>) -> Self {
        let (parts, body) = response.into_parts();
        Self {
            parts,
            body: Some(body),
        }
    }
}

#[pymethods]
impl ClientResponse {
    fn read(&mut self) -> PyResult<PyObject> {
        let (fut, res_fut, loop_) = create_future()?;

        // TODO: Error when already taken
        let body = match self.body.take() {
            Some(b) => b,
            None => return Err(Error::new_err("Response body has already been read")),
        };

        RUNTIME.spawn(async move {
            let body = aggregate(body).await;

            match body {
                Ok(b) => {
                    let chunk = b.chunk();
                    let gil = Python::acquire_gil();
                    let py = gil.python();
                    let _ = set_fut_result(loop_, fut, chunk.into_py(py));
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

        // TODO: Error when already taken
        let body = match self.body.take() {
            Some(b) => b,
            None => return Err(Error::new_err("Response body has already been read")),
        };

        RUNTIME.spawn(async move {
            let body = aggregate(body).await;

            match body {
                Ok(b) => {
                    let chunk = b.chunk();

                    match from_utf8(chunk) {
                        Ok(string) => {
                            let gil = Python::acquire_gil();
                            let py = gil.python();
                            let _ = set_fut_result(loop_, fut, string.into_py(py));
                        }
                        Err(e) => {
                            let _ = set_fut_exc(loop_, fut, Error::new_err(e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    let _ = set_fut_exc(loop_, fut, Error::new_err(e.to_string()));
                }
            }
        });

        Ok(res_fut)
    }
}
