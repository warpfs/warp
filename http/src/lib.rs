pub use self::request::*;
pub use self::response::*;
use http::Method;
use thiserror::Error;
use url::Url;

mod request;
mod response;
#[cfg(windows)]
mod winhttp;

/// Struct to construct [`Request`] objects.
pub struct HttpClient {
    #[cfg(windows)]
    session: winhttp::Handle,
}

impl HttpClient {
    pub fn new() -> Result<Self, NewError> {
        Ok(Self {
            #[cfg(windows)]
            session: Self::winhttp_open(None).map_err(NewError::CreateWinHttpSessionFailed)?,
        })
    }

    pub fn request(&self, method: Method, url: impl AsRef<Url>) -> Result<Request, RequestError> {
        let url = url.as_ref();

        if !matches!(url.scheme(), "http" | "https") {
            return Err(RequestError::NotHttp);
        }

        Request::new(
            method,
            url,
            #[cfg(windows)]
            &self.session,
        )
    }

    #[cfg(windows)]
    fn winhttp_open(agent: Option<&str>) -> Result<winhttp::Handle, std::io::Error> {
        use std::ptr::null;
        use windows_sys::Win32::Networking::WinHttp::{
            WinHttpOpen, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
        };

        // Encode agent.
        let agent = agent.map(|v| {
            let mut v: Vec<u16> = v.encode_utf16().collect();
            v.push(0);
            v
        });

        // Create WinHTTP session.
        let session = unsafe {
            WinHttpOpen(
                agent.map(|v| v.as_ptr()).unwrap_or(null()),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                null(),
                null(),
                0,
            )
        };

        if session.is_null() {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(unsafe { winhttp::Handle::new(session) })
        }
    }
}

#[cfg(windows)]
unsafe impl Send for HttpClient {}

#[cfg(windows)]
unsafe impl Sync for HttpClient {}

/// Represents an error when [`HttpClient::new()`] fails.
#[derive(Debug, Error)]
pub enum NewError {
    #[cfg(windows)]
    #[error("couldn't create WinHTTP session")]
    CreateWinHttpSessionFailed(#[source] std::io::Error),
}

/// Represents an error when [`HttpClient::request()`] fails.
#[derive(Debug, Error)]
pub enum RequestError {
    #[error("the URL is not a HTTP URL")]
    NotHttp,

    #[error("the specified method is not supported")]
    UnsupportedMethod,

    #[cfg(windows)]
    #[error("WinHttpConnect was failed")]
    WinHttpConnectFailed(#[source] std::io::Error),

    #[cfg(windows)]
    #[error("WinHttpOpenRequest was failed")]
    WinHttpOpenRequestFailed(#[source] std::io::Error),

    #[cfg(windows)]
    #[error("WinHttpAddRequestHeaders was failed ({0})")]
    WinHttpAddRequestHeadersFailed(&'static str, #[source] std::io::Error),
}
