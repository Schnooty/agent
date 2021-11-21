use http::request::Request;
use http::response::Response;
use http::uri::Scheme;
use http::status::StatusCode;
use http::header::HeaderName;
use http::header::HeaderValue;
use async_std::net::TcpStream;
use std::io::Write;
use std::io;
use crate::futures::AsyncReadExt;
use crate::futures::AsyncWriteExt as AsyncWrite;
use http::header::HeaderMap;
use log::trace;
use http::response::Builder;
use std::time::Duration;

pub struct HttpClient {
    request: Request<String>
}

#[derive(Default, Debug)]
pub struct HttpOptions {
    pub timeout: Option<Duration>,
    pub http_proxy: Option<(String, u16)>,
    pub https_proxy: Option<(String, u16)>
}

const HTTP_VERSION: &'static str = "HTTP/1.1";
const SPACE: u8 = ' ' as u8;
const NL: u8 = '\n' as u8;
const CR: u8 = '\r' as u8;
const COLON: u8 = ':' as u8;
const HOST: &'static str = "Host";
const HTTP_PORT: u16 = 80;
const HTTPS_PORT: u16 = 443;

impl HttpClient {
    pub fn new(request: Request<String>) -> Self {
        Self {
            request
        }
    }

    pub async fn send(&self) -> Result<Response<Vec<u8>>, HttpError> {
        let request = &self.request;
        let uri = request.uri();
        let mut req_bytes = Vec::new();

        let body_bytes = self.request.body().as_bytes();

        debug!("Starting request: {} {}", uri, request.method());

        let host = match uri.host() {
            Some(h) => h,
            None => return Err(HttpError::MissingHost)
        };

        Write::write_all(&mut req_bytes, request.method().as_ref().as_bytes())?; 
        Write::write_all(&mut req_bytes, &[SPACE])?;
        Write::write_all(&mut req_bytes, uri.path().as_ref())?; // TODO IS IT ENCODED?
        Write::write_all(&mut req_bytes, &[SPACE])?;
        Write::write_all(&mut req_bytes, HTTP_VERSION.as_bytes())?;
        Write::write_all(&mut req_bytes, &[NL])?;

        let headers = self.request.headers();

        if !headers.contains_key(HOST) {
            Write::write_all(&mut req_bytes, &format!("Host: {}\r\n", host).into_bytes())?;
        }

        Write::write_all(&mut req_bytes, &format!("Connection: Close\r\n").into_bytes())?;
        Write::write_all(&mut req_bytes, &format!("Content-Length: {}\r\n", body_bytes.len()).into_bytes())?;

        for (h, v) in headers {
            Write::write_all(&mut req_bytes, h.as_ref())?;
            Write::write_all(&mut req_bytes, &format!(": ",).as_bytes())?;
            Write::write_all(&mut req_bytes, v.as_bytes())?;
            Write::write_all(&mut req_bytes, &[NL])?;
        }

        Write::write_all(&mut req_bytes, &[NL])?;
        Write::write_all(&mut req_bytes, body_bytes)?;

        let response: Vec<u8> = if uri.scheme() == Some(&Scheme::HTTP) {
            let port = match uri.port() {
                Some(p) => p.as_u16(),
                None => HTTP_PORT
            };
            trace!("Opening HTTP connection on port {}", port);

            let auth = format!("{}:{}", host, port);
            let mut stream = TcpStream::connect(auth).await?;

            trace!("Writing {} byte(s)", req_bytes.len());
            AsyncWrite::write_all(&mut stream, &mut req_bytes).await?;

            trace!("Reading response");
            let mut res_bytes = Vec::new();
            stream.read_to_end(&mut res_bytes).await?;

            debug!("Successfully read {} byte(s)", res_bytes.len());
            res_bytes
        } else if uri.scheme() == Some(&Scheme::HTTPS) {
            let port = match uri.port() {
                Some(p) => p.as_u16(),
                None => HTTPS_PORT
            };
            trace!("Opening HTTP connection on port {}", port);
            let auth = format!("{}:{}", host, port);
            let stream = TcpStream::connect(auth).await?;
            let mut tls_stream = async_native_tls::connect(host, stream).await?;

            trace!("Writing {} byte(s)", req_bytes.len());
            trace!("Full request: {}", String::from_utf8_lossy(&req_bytes));
            AsyncWrite::write_all(&mut tls_stream, &mut req_bytes).await?;

            debug!("Reading response");
            let mut res_bytes = Vec::new();
            tls_stream.read_to_end(&mut res_bytes).await?;

            debug!("Successfully read response");
            res_bytes
        } else {
            todo!()
        };

        trace!("Entire response: {}", String::from_utf8_lossy(&response));

        HttpParser::new(&response).parse_response()
    }
}

#[derive(Debug)]
pub enum HttpError {
    MissingHost,
    IOErr,
    TlsErr,
    ResponseParseErr
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<io::Error> for HttpError {
    fn from(_: io::Error) -> Self {
        HttpError::IOErr
    }
}

impl From<hyper_tls::Error> for HttpError {
    fn from(_: hyper_tls::Error) -> Self {
        HttpError::TlsErr
    }
}

impl From<std::str::Utf8Error> for HttpError {
    fn from(_: std::str::Utf8Error) -> Self {
        HttpError::ResponseParseErr
    }
}

impl From<std::num::ParseIntError> for HttpError {
    fn from(_: std::num::ParseIntError) -> Self {
        HttpError::ResponseParseErr
    }
}

impl From<http::header::InvalidHeaderName> for HttpError {
    fn from(_: http::header::InvalidHeaderName) -> Self {
        HttpError::ResponseParseErr
    }
}

impl From<http::header::InvalidHeaderValue> for HttpError {
    fn from(_: http::header::InvalidHeaderValue) -> Self {
        HttpError::ResponseParseErr
    }
}

impl From<http::status::InvalidStatusCode> for HttpError {
    fn from(_: http::status::InvalidStatusCode) -> Self {
        HttpError::ResponseParseErr
    }
}

impl From<http::Error> for HttpError {
    fn from(_: http::Error) -> Self {
        todo!()
    }
}

struct HttpParser<'a> {
    index: usize,
    input: &'a [u8],
    status: Option<StatusCode>,
    header_map: HeaderMap,
    body: Vec<u8>
}

impl<'a> HttpParser<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self {
            index: 0,
            input,
            status: None,
            header_map: HeaderMap::new(),
            body: Vec::new()
        }
    }

    fn parse_response(mut self) -> Result<Response<Vec<u8>>, HttpError> {
        trace!("Parsing response");
        trace!("Parsing status line");
        self.parse_status_line()?;
        trace!("Parsing headers");
        self.parse_headers()?;
        self.parse_newline()?;
        trace!("Parsing body");
        self.parse_body()?;
        let mut builder = Builder::new();
        builder = builder.status(self.status.unwrap());
        for (k, v) in self.header_map.iter() {
            builder = builder.header(k, v);
        }
        trace!("Successfully parsed response");
        Ok(builder.body(self.body)?)
    }

    fn parse_status_line(&mut self) -> Result<(), HttpError> {
        self.parse_protocol()?;
        self.parse_space()?;
        self.parse_status_code()?;
        self.parse_space()?;
        self.parse_status_message()?;
        self.parse_newline()?;
        Ok(())
    }

    fn parse_space(&mut self) -> Result<(), HttpError> {
        self.eat(" ")
    }

    fn parse_newline(&mut self) -> Result<(), HttpError> {
        self.eat("\r\n")
    }

    fn parse_protocol(&mut self) -> Result<(), HttpError> {
        self.eat("HTTP/1.1")
    }

    fn parse_status_message(&mut self) -> Result<(), HttpError> {
        let mut i = self.index;
        while self.input[i] != CR && i < self.input.len() {
            i += 1;
        }
        self.index = i; 
        Ok(())
    }

    fn parse_status_code(&mut self) -> Result<StatusCode, HttpError> {
        let start = self.index;
        while self.input[self.index] != SPACE && self.index < self.input.len() {
            self.index += 1;
        }
        let sstr = &std::str::from_utf8(&self.input[start..self.index])?;
        let status: StatusCode = sstr.parse()?;
        //self.parse_newline()?;
        self.status = Some(status);
        Ok(status)
    }

    fn parse_headers(&mut self) -> Result<(), HttpError> {
        while !self.is_empty_line() {
            self.parse_header()?;
        }
        Ok(())
    }

    fn is_empty_line(&self) -> bool {
        self.input.len() - self.index >= 2 && self.input[self.index] == CR && self.input[self.index + 1] == NL
    }

    fn parse_header(&mut self) -> Result<(HeaderName, HeaderValue), HttpError> {
        let header_name = self.parse_header_name()?;
        self.eat(":")?;
        let header_value = self.parse_header_value()?;
        self.header_map.append(header_name.clone(), header_value.clone());
        self.parse_newline()?;
        Ok((header_name, header_value))
    }

    fn parse_header_name(&mut self) -> Result<HeaderName, HttpError> {
        let start = self.index;
        while self.index < self.input.len() && self.input[self.index] != COLON {
            self.index += 1;
        }
        Ok(std::str::from_utf8(&self.input[start..self.index])?.parse()?)
    }

    fn parse_header_value(&mut self) -> Result<HeaderValue, HttpError> {
        let mut start = self.index;
        while start < self.input.len() && self.input[start] == SPACE {
            start += 1;
        }

        while self.index < self.input.len() && self.input[self.index] != CR {
            self.index += 1;
        }
        Ok(std::str::from_utf8(&self.input[start..self.index])?.parse()?)
    }

    fn parse_body(&mut self) -> Result<Vec<u8>, HttpError> {
        self.body = (&self.input[self.index..self.input.len()])
            .iter()
            .cloned()
            .collect();
        Ok(self.body.clone())
    }

    fn eat(&mut self, s: &str) -> Result<(), HttpError> {
        let bytes = s.as_bytes();
        if self.index + bytes.len() > self.input.len() {
            trace!("Unable to parse {:?} at index {} (length {}) - insufficient length", s, self.index, self.input.len());
            return Err(HttpError::ResponseParseErr);
        }
        for b in bytes {
            if self.input[self.index] != *b {
                return Err(HttpError::ResponseParseErr);
            }
            self.index += 1;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use actix_rt;
    use crate::http::HttpClient;
    use crate::http::HttpParser;
    use http::HeaderMap;
    use http::Response;
    use http::header::HeaderValue;
    use http::status::StatusCode;
    use http::Request;

    #[test]
    fn parses_very_simple_response() {
        let resp = parse_response("HTTP/1.1 200 OK\r\n\r\n");

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn parses_simple_response_two_headers() {
        let resp = parse_response("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\n\r\nHi mum!");

        assert_eq!(resp.status(), StatusCode::OK);
        assert_header(resp.headers(), "Content-Type", "text/plain");
        assert_body(&resp, "Hi mum!");
    }

    #[test]
    fn parses_simple_response_no_body() {
        let resp = parse_response("HTTP/1.1 301 OK\r\nLocation: http://website.thing\r\n\r\n");

        assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
        assert_header(resp.headers(), "Location", "http://website.thing");
        assert_eq!(resp.body().len(), 0);
    }

    #[test]
    fn parses_simple_response_empty_headers() {
        let resp = parse_response("HTTP/1.1 200 OK\r\nEnjoy:\r\n\r\nHello there");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_header(resp.headers(), "Enjoy", "");
        assert_body(&resp, "Hello there");
    }

    #[test]
    fn parses_405_response_no_body() {
        let resp = parse_response("HTTP/1.1 405 Method Not Allowed\r\ncontent-length: 0\r\ndate: Mon, 15 Nov 2021 19:18:35 GMT\r\n\r\n");
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_header(resp.headers(), "Content-Length", "0");
    }

    #[actix_rt::test]
    async fn do_google() {
        let resp = HttpClient::new(Request::builder()
                .uri("https://www.google.com")
                .body(String::new())
                .unwrap()
            )
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    fn parse_response(text: &str) -> Response<Vec<u8>> {
        HttpParser::new(text.as_bytes()).parse_response().unwrap()
    }

    fn assert_header(headers: &HeaderMap, key: &str, value: &str) {
        let actual_header: &HeaderValue = headers 
            .get(key)
            .clone()
            .unwrap();
        let expected_header: HeaderValue = value.parse().unwrap();
        assert_eq!(actual_header, &expected_header);
    }

    fn assert_body(resp: &Response<Vec<u8>>, body: &str) {
        assert_eq!(String::from_utf8(resp.body().clone()).unwrap(), body);
    }
}
