use pyo3::{
    prelude::{pyclass, pyproto},
    PyObjectProtocol,
};

#[pyclass]
pub struct HttpVersion {
    #[pyo3(get)]
    major: u8,
    #[pyo3(get)]
    minor: u8,
}

impl From<http::Version> for HttpVersion {
    fn from(version: http::Version) -> Self {
        if version == http::Version::HTTP_09 {
            Self { major: 0, minor: 9 }
        } else if version == http::Version::HTTP_10 {
            Self { major: 1, minor: 0 }
        } else if version == http::Version::HTTP_11 {
            Self { major: 1, minor: 1 }
        } else if version == http::Version::HTTP_2 {
            Self { major: 2, minor: 0 }
        } else if version == http::Version::HTTP_3 {
            Self { major: 3, minor: 0 }
        } else {
            unreachable!()
        }
    }
}

#[pyproto]
impl PyObjectProtocol for HttpVersion {
    fn __repr__(&self) -> String {
        format!("HttpVersion(major={}, minor={})", self.major, self.minor)
    }
}
