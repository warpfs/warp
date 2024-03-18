use crate::{HttpClient, RequestError, Response};
use http::Method;
use std::{marker::PhantomData, rc::Rc};
use thiserror::Error;
use url::Url;

/// HTTP request.
pub struct Request<'a> {
    #[cfg(unix)]
    session: curl::easy::Easy2<Handler>,
    #[cfg(windows)]
    request: crate::winhttp::Handle, // Must be dropped before connection.
    #[cfg(windows)]
    connection: crate::winhttp::Handle, // Must be dropped last.
    phantom: PhantomData<&'a HttpClient>,
}

impl<'a> Request<'a> {
    #[cfg(unix)]
    pub(crate) fn new(method: Method, url: &Url) -> Result<Self, RequestError> {
        use curl::easy::Easy2;

        // Remove username and password.
        let mut url = url.clone();

        url.set_username("").unwrap();
        url.set_password(None).unwrap();

        // Create CURL session.
        let mut session = Easy2::new(Handler {
            phantom: PhantomData,
        });

        session.url(url.as_str()).unwrap();
        session.follow_location(true).unwrap();

        match method {
            Method::DELETE | Method::PATCH => session.custom_request(method.as_str()).unwrap(),
            Method::GET => session.get(true).unwrap(),
            Method::POST => session.post(true).unwrap(),
            Method::PUT => session.put(true).unwrap(),
            _ => return Err(RequestError::UnsupportedMethod),
        }

        Ok(Self {
            session,
            phantom: PhantomData,
        })
    }

    #[cfg(windows)]
    pub(crate) fn new(
        method: Method,
        url: &Url,
        session: &'a crate::winhttp::Handle,
    ) -> Result<Self, RequestError> {
        use std::io::Error;
        use std::ptr::null;
        use windows_sys::w;
        use windows_sys::Win32::Networking::WinHttp::{
            WinHttpConnect, WinHttpOpenRequest, WINHTTP_FLAG_ESCAPE_DISABLE,
            WINHTTP_FLAG_ESCAPE_DISABLE_QUERY, WINHTTP_FLAG_SECURE,
        };

        // Get method.
        let method = match method {
            Method::DELETE => w!("DELETE"),
            Method::GET => w!("GET"),
            Method::PATCH => w!("PATCH"),
            Method::POST => w!("POST"),
            Method::PUT => w!("PUT"),
            _ => return Err(RequestError::UnsupportedMethod),
        };

        // Is it possible for HTTP URL without host?
        let host = url.host_str().unwrap();
        let port = url.port_or_known_default().unwrap();
        let secure = if url.scheme() == "https" {
            WINHTTP_FLAG_SECURE
        } else {
            0
        };

        // Encode host.
        let mut host: Vec<u16> = host.encode_utf16().collect();

        host.push(0);

        // Create connection handle.
        let connection = unsafe { WinHttpConnect(session.get(), host.as_ptr(), port, 0) };

        if connection.is_null() {
            return Err(RequestError::WinHttpConnectFailed(Error::last_os_error()));
        }

        // Concat path and query.
        let connection = unsafe { crate::winhttp::Handle::new(connection) };
        let mut path = url.path().to_owned();

        if let Some(v) = url.query() {
            path.push('?');
            path.push_str(v);
        }

        // Encode path.
        let mut path: Vec<u16> = path.encode_utf16().collect();

        path.push(0);

        // Setup accept list.
        let mut accept: Vec<*const u16> = Vec::new();

        accept.push(w!("*/*"));
        accept.push(null());

        // Create request handle.
        let request = unsafe {
            WinHttpOpenRequest(
                connection.get(),
                method,
                path.as_ptr(),
                null(),
                null(),
                accept.as_ptr(),
                WINHTTP_FLAG_ESCAPE_DISABLE | WINHTTP_FLAG_ESCAPE_DISABLE_QUERY | secure,
            )
        };

        if request.is_null() {
            Err(RequestError::WinHttpOpenRequestFailed(
                Error::last_os_error(),
            ))
        } else {
            Ok(Self {
                request: unsafe { crate::winhttp::Handle::new(request) },
                connection,
                phantom: PhantomData,
            })
        }
    }

    #[cfg(unix)]
    pub fn send(self) -> Result<Response, SendError> {
        // Execute the request.
        self.session
            .perform()
            .map_err(SendError::CurlPerformFailed)?;

        Ok(Response::new())
    }

    #[cfg(windows)]
    pub fn send(self) -> Result<Response, SendError> {
        use std::io::Error;
        use std::ptr::null;
        use windows_sys::Win32::Foundation::FALSE;
        use windows_sys::Win32::Networking::WinHttp::WinHttpSendRequest;

        if unsafe { WinHttpSendRequest(self.request.get(), null(), 0, null(), 0, 0, 0) } == FALSE {
            return Err(SendError::WinHttpSendRequestFailed(Error::last_os_error()));
        }

        Ok(Response::new())
    }
}

/// An implementation of [`curl::easy::Handler`].
#[cfg(unix)]
struct Handler {
    phantom: PhantomData<Rc<()>>, // !Send & !Sync.
}

#[cfg(unix)]
impl curl::easy::Handler for Handler {}

/// Represents an error when [`Request::send()`] fails.
#[derive(Debug, Error)]
pub enum SendError {
    #[cfg(unix)]
    #[error("curl_easy_perform was failed")]
    CurlPerformFailed(#[source] curl::Error),

    #[cfg(windows)]
    #[error("WinHttpSendRequest was failed")]
    WinHttpSendRequestFailed(#[source] std::io::Error),
}
