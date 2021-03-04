use http::{HeaderMap, Method, StatusCode, Version};
use pyo3::{
    exceptions::PyValueError,
    prelude::{pyclass, pymethods},
    types::{PyAny, PyDict},
    AsPyPointer, IntoPy, PyObject, PyResult, Python,
};
use reqwest::{multipart::Form, Client, Response, Url};

use std::{collections::HashMap, convert::TryFrom};

use crate::{
    asyncio::{create_future, set_fut_exc, set_fut_result},
    error::Error,
    orjson::{dumps, loads},
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

        let url = match Url::parse(url) {
            Ok(mut u) => {
                if let Some(kwargs) = &kwargs {
                    if kwargs.contains("params")? {
                        let params = kwargs.get_item("params").unwrap();
                        let dict: &PyDict = params.cast_as()?;
                        let mut uri_params = u.query_pairs_mut();

                        for (key, value) in dict.iter() {
                            uri_params.append_pair(&key.to_string(), &value.to_string());
                        }
                    }
                }

                u
            }
            Err(e) => return Err(PyValueError::new_err(e.to_string())),
        };

        // TODO: Cookies

        let mut builder = self.client.request(method, url);

        if let Some(kwargs) = kwargs {
            if kwargs.contains("json")? {
                let json = kwargs.get_item("json").unwrap();
                let serialized = unsafe { dumps(json.as_ptr()) }?;
                println!("using json: {:?}", &serialized);
                builder = builder
                    .body(serialized)
                    .header("Content-Type", "application/json");
            }

            if kwargs.contains("data")? {
                let data = kwargs.get_item("data").unwrap();
                let dict: &PyDict = data.cast_as()?;

                let mut form = Form::new();

                for (key, value) in dict.iter() {
                    form = form.text(key.to_string(), value.to_string());
                }

                builder = builder.multipart(form);
            }

            if kwargs.contains("headers")? {
                let headers = kwargs.get_item("headers").unwrap();
                let dict: &PyDict = headers.cast_as()?;

                for (key, value) in dict.iter() {
                    builder = builder.header(&key.to_string(), value.to_string());
                }
            }
        }

        let req = builder.build().unwrap();
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

    fn json(&mut self) -> PyResult<PyObject> {
        let (fut, res_fut, loop_) = create_future()?;

        let body = match self.response.take() {
            Some(b) => b,
            None => return Err(Error::new_err("Response body has already been read")),
        };

        RUNTIME.spawn(async move {
            let bytes = body.bytes().await;

            match bytes {
                Ok(bytes) => {
                    println!("before: {}", std::str::from_utf8(&bytes).unwrap());
                    let json = unsafe { loads(&bytes) };
                    println!("after");

                    match json {
                        Ok(json) => {
                            let gil = Python::acquire_gil();
                            let py = gil.python();
                            let res: &PyAny = unsafe { py.from_owned_ptr(json) };
                            let sr = fut.getattr(py, "set_result").unwrap();

                            let _ = loop_.call_method1(py, "call_soon_threadsafe", (sr, res));
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

    #[getter]
    fn status(&self) -> u16 {
        self.status.as_u16()
    }

    #[getter]
    fn version(&self) -> HttpVersion {
        HttpVersion::from(self.version)
    }

    #[getter]
    fn url(&self) -> String {
        self.url.to_string()
    }

    #[getter]
    fn ok(&self) -> bool {
        400 > self.status.as_u16()
    }

    #[getter]
    fn headers(&self) -> HashMap<String, String> {
        self.headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_owned()))
            .collect()
    }
}
