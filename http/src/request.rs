use crate::{HttpClient, Response};
use mime::Mime;
use std::cmp::min;
use std::io::Read;
use std::ops::SubAssign;
use thiserror::Error;
use url::Url;

/// HTTP request.
pub struct Request<'a> {
    #[cfg(windows)]
    client: &'a HttpClient,
    url: &'a Url,
}

impl<'a> Request<'a> {
    #[cfg(unix)]
    pub(crate) fn new(_: &'a HttpClient, url: &'a Url) -> Self {
        Self { url }
    }

    #[cfg(windows)]
    pub(crate) fn new(client: &'a HttpClient, url: &'a Url) -> Self {
        Self { client, url }
    }

    #[cfg(unix)]
    pub fn exec<B: Read>(self, method: Method<B>) -> Result<Response, ExecError> {
        use curl::easy::{Easy2, List};

        // Create CURL session.
        let mut session = Easy2::new(Handler {
            request: None,
            error: None,
        });

        if !self.url.username().is_empty() {
            todo!()
        }

        if self.url.password().is_some() {
            todo!()
        }

        session.url(self.url.as_str()).unwrap();
        session.path_as_is(true).unwrap();
        session.autoreferer(true).unwrap();
        session.follow_location(true).unwrap();

        // Set method.
        match method {
            Method::Post(body) => {
                let mut headers = List::new();

                headers
                    .append(format!("Content-Type: {}", body.ty).as_str())
                    .unwrap();

                session.post(true).unwrap();
                session.post_field_size(body.len).unwrap();
                session.http_headers(headers).unwrap();
                session.get_mut().request = Some((body.content, body.len));
            }
        }

        // Execute the request.
        session.perform().map_err(|e| {
            if e.is_aborted_by_callback() {
                ExecError::ReadRequestFailed(session.get_mut().error.take().unwrap())
            } else {
                ExecError::CurlPerformFailed(e)
            }
        })?;

        Ok(Response::new())
    }

    #[cfg(windows)]
    pub fn exec<B: Read>(self, method: Method<B>) -> Result<Response, ExecError> {
        use crate::winhttp::Handle;
        use std::io::{Error, ErrorKind};
        use std::ptr::null;
        use windows_sys::w;
        use windows_sys::Win32::Foundation::FALSE;
        use windows_sys::Win32::Networking::WinHttp::{
            WinHttpAddRequestHeaders, WinHttpConnect, WinHttpOpenRequest, WinHttpSendRequest,
            WinHttpWriteData, WINHTTP_ADDREQ_FLAG_ADD, WINHTTP_ADDREQ_FLAG_REPLACE,
            WINHTTP_FLAG_ESCAPE_DISABLE, WINHTTP_FLAG_ESCAPE_DISABLE_QUERY, WINHTTP_FLAG_SECURE,
            WINHTTP_IGNORE_REQUEST_TOTAL_LENGTH,
        };

        if !self.url.username().is_empty() {
            todo!()
        }

        if self.url.password().is_some() {
            todo!()
        }

        // Is it possible for HTTP URL without host?
        let host = self.url.host_str().unwrap();
        let port = self.url.port_or_known_default().unwrap();

        // Encode host.
        let mut host: Vec<u16> = host.encode_utf16().collect();

        host.push(0);

        // Create connection handle.
        let connection =
            unsafe { WinHttpConnect(self.client.session.get(), host.as_ptr(), port, 0) };

        if connection.is_null() {
            return Err(ExecError::WinHttpConnectFailed(Error::last_os_error()));
        }

        // Concat path and query.
        let connection = unsafe { Handle::new(connection) };
        let mut path = self.url.path().to_owned();

        if let Some(v) = self.url.query() {
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

        // Get HTTPS flags.
        let secure = if self.url.scheme() == "https" {
            WINHTTP_FLAG_SECURE
        } else {
            0
        };

        // Create request handle.
        let request = unsafe {
            WinHttpOpenRequest(
                connection.get(),
                match &method {
                    Method::Post(_) => w!("POST"),
                },
                path.as_ptr(),
                null(),
                null(),
                accept.as_ptr(),
                WINHTTP_FLAG_ESCAPE_DISABLE | WINHTTP_FLAG_ESCAPE_DISABLE_QUERY | secure,
            )
        };

        if request.is_null() {
            return Err(ExecError::WinHttpOpenRequestFailed(Error::last_os_error()));
        }

        // Setup headers.
        let request = unsafe { Handle::new(request) };
        let len = match &method {
            Method::Post(body) => {
                // Set Content-Type.
                let header: Vec<u16> = format!("Content-Type: {}", body.ty)
                    .encode_utf16()
                    .collect();

                if unsafe {
                    WinHttpAddRequestHeaders(
                        request.get(),
                        header.as_ptr(),
                        header.len().try_into().unwrap(),
                        WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE,
                    )
                } == FALSE
                {
                    return Err(ExecError::SetContentTypeFailed(Error::last_os_error()));
                }

                // Set Content-Length.
                match TryInto::<u32>::try_into(body.len) {
                    Ok(v) => v,
                    Err(_) => {
                        let header: Vec<u16> = format!("Content-Length: {}", body.len)
                            .encode_utf16()
                            .collect();

                        if unsafe {
                            WinHttpAddRequestHeaders(
                                request.get(),
                                header.as_ptr(),
                                header.len().try_into().unwrap(),
                                WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE,
                            )
                        } == FALSE
                        {
                            return Err(ExecError::SetContentLengthFailed(Error::last_os_error()));
                        }

                        WINHTTP_IGNORE_REQUEST_TOTAL_LENGTH
                    }
                }
            }
        };

        // Send the request.
        if unsafe { WinHttpSendRequest(request.get(), null(), 0, null(), 0, len, 0) } == FALSE {
            return Err(ExecError::WinHttpSendRequestFailed(Error::last_os_error()));
        }

        // Write the body.
        match method {
            Method::Post(mut body) => {
                let mut remaining = body.len;
                let mut buf = vec![0u8; min(body.len, 1024 * 1024).try_into().unwrap()];

                while remaining != 0 {
                    // Read.
                    let len = min(buf.len(), remaining.try_into().unwrap_or(usize::MAX));
                    let read = match body.content.read(&mut buf[..len]) {
                        Ok(v) => v,
                        Err(e) => {
                            if e.kind() == ErrorKind::Interrupted {
                                continue;
                            }

                            return Err(ExecError::ReadRequestFailed(e));
                        }
                    };

                    if read == 0 {
                        return Err(ExecError::ReadRequestFailed(Error::from(
                            ErrorKind::UnexpectedEof,
                        )));
                    }

                    // Write.
                    let mut buf = &buf[..read];

                    while !buf.is_empty() {
                        let mut sent = 0;

                        if unsafe {
                            WinHttpWriteData(
                                request.get(),
                                buf.as_ptr().cast(),
                                buf.len().try_into().unwrap(),
                                &mut sent,
                            )
                        } == FALSE
                        {
                            return Err(ExecError::WinHttpWriteDataFailed(Error::last_os_error()));
                        }

                        buf = &buf[sent.try_into().unwrap()..];
                    }

                    remaining.sub_assign(TryInto::<u64>::try_into(read).unwrap());
                }
            }
        }

        Ok(Response::new())
    }
}

/// An implementation of [`curl::easy::Handler`].
#[cfg(unix)]
struct Handler<R> {
    request: Option<(R, u64)>,
    error: Option<std::io::Error>,
}

#[cfg(unix)]
impl<R: Read> curl::easy::Handler for Handler<R> {
    fn read(&mut self, data: &mut [u8]) -> Result<usize, curl::easy::ReadError> {
        use curl::easy::ReadError;
        use std::io::{Error, ErrorKind};

        // Check if the request has body.
        let (body, remaining) = match self.request.as_mut() {
            Some(v) => v,
            None => return Ok(0),
        };

        if *remaining == 0 {
            return Ok(0);
        }

        // Read the body.
        let len = min(data.len(), (*remaining).try_into().unwrap_or(usize::MAX));
        let read = loop {
            match body.read(&mut data[..len]) {
                Ok(v) => break v,
                Err(e) => {
                    if e.kind() == ErrorKind::Interrupted {
                        continue;
                    }

                    self.error = Some(e);
                    return Err(ReadError::Abort);
                }
            }
        };

        if read == 0 {
            self.error = Some(Error::from(ErrorKind::UnexpectedEof));
            return Err(ReadError::Abort);
        }

        remaining.sub_assign(TryInto::<u64>::try_into(read).unwrap());

        Ok(read)
    }
}

/// Method of the request.
pub enum Method<'a, B> {
    Post(RequestBody<'a, B>),
}

/// Body of the request.
pub struct RequestBody<'a, C> {
    ty: &'a Mime,
    len: u64,
    content: C,
}

impl<'a, C> RequestBody<'a, C> {
    pub fn new(ty: &'a Mime, len: u64, content: C) -> Self {
        Self { ty, len, content }
    }
}

/// Represents an error when [`Request::exec()`] fails.
#[derive(Debug, Error)]
pub enum ExecError {
    #[error("couldn't read the request body")]
    ReadRequestFailed(#[source] std::io::Error),

    #[cfg(unix)]
    #[error("curl_easy_perform was failed")]
    CurlPerformFailed(#[source] curl::Error),

    #[cfg(windows)]
    #[error("WinHttpConnect was failed")]
    WinHttpConnectFailed(#[source] std::io::Error),

    #[cfg(windows)]
    #[error("WinHttpOpenRequest was failed")]
    WinHttpOpenRequestFailed(#[source] std::io::Error),

    #[cfg(windows)]
    #[error("couldn't set Content-Type")]
    SetContentTypeFailed(#[source] std::io::Error),

    #[cfg(windows)]
    #[error("couldn't set Content-Length")]
    SetContentLengthFailed(#[source] std::io::Error),

    #[cfg(windows)]
    #[error("WinHttpSendRequest was failed")]
    WinHttpSendRequestFailed(#[source] std::io::Error),

    #[cfg(windows)]
    #[error("WWinHttpWriteData was failed")]
    WinHttpWriteDataFailed(#[source] std::io::Error),
}
