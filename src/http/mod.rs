use reqwest::{Error, Request, Response};
use std::io;

pub struct HttpClient {
    request: Request,
}

impl HttpClient {
    pub fn new(request: Request) -> Self {
        Self { request }
    }

    pub async fn send(self) -> Result<Response, HttpError> {
        let method = self.request.method().clone();
        let url = self.request.url().clone();
        debug!("Sending request {} {}", method, url);
        let response = reqwest::Client::new().execute(self.request).await?;
        debug!("Got response {} for {} {}", response.status(), method, url);
        Ok(response)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum HttpError {
    Unknown,
    //MissingHost,
    IOErr,
    TlsErr,
    ResponseParseErr,
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

impl From<Error> for HttpError {
    fn from(err: Error) -> Self {
        error!("HTTP error: {}", err);
        HttpError::Unknown
    }
}

/*impl From<http::header::InvalidHeaderName> for HttpError {
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
}*/

/*struct HttpParser<'a, R: AsyncReadExt> {
    index: usize,
    buffer: Vec<u8>,
    source: &'a mut R,
    status: Option<StatusCode>,
    header_map: HeaderMap,
    body: Vec<u8>,
    eof: bool,
    end: usize,
}*/

//const DEFAULT_BUFFER_SIZE: usize = 1024;

/*impl<'a, R: AsyncReadExt + Unpin> HttpParser<'a, R> {
    fn new(source: &'a mut R) -> Self {
        Self::with_options(source, DEFAULT_BUFFER_SIZE)
    }

    fn with_options(source: &'a mut R, buffer_size: usize) -> Self {
        let buffer = Vec::with_capacity(buffer_size);

        Self {
            index: 0,
            end: 0,
            source,
            buffer,
            status: None,
            header_map: HeaderMap::new(),
            body: Vec::new(),
            eof: false,
        }
    }

    async fn parse_response(mut self) -> Result<Response<Vec<u8>>, HttpError> {
        self.parse_status_line().await?;

        self.parse_headers().await?;
        self.parse_newline().await?;

        self.parse_body().await?;

        let mut builder = Builder::new();
        builder = builder.status(self.status.unwrap());
        for (k, v) in self.header_map.iter() {
            builder = builder.header(k, v);
        }
        Ok(builder.body(self.body)?)
    }

    async fn parse_status_line(&mut self) -> Result<(), HttpError> {
        self.parse_protocol().await?;
        self.parse_space().await?;
        self.parse_status_code().await?;
        self.parse_space().await?;
        self.parse_status_message().await?;
        self.parse_newline().await?;
        Ok(())
    }

    async fn parse_space(&mut self) -> Result<(), HttpError> {
        self.eat(" ").await
    }

    async fn parse_newline(&mut self) -> Result<(), HttpError> {
        self.eat("\r\n").await
    }

    async fn parse_protocol(&mut self) -> Result<(), HttpError> {
        self.eat("HTTP/1.1").await
    }

    async fn parse_status_message(&mut self) -> Result<(), HttpError> {
        if !self.advance_until('\r').await? {
            Err(HttpError::ResponseParseErr)
        } else {
            Ok(())
        }
    }

    async fn parse_status_code(&mut self) -> Result<StatusCode, HttpError> {
        let start = self.index;
        if !self.advance_until(' ').await? {
            return Err(HttpError::ResponseParseErr);
        }
        let sstr = &std::str::from_utf8(&self.buffer[start..self.index])?;
        self.status = Some(sstr.parse()?);
        Ok(sstr.parse()?)
    }

    async fn parse_headers(&mut self) -> Result<(), HttpError> {
        while !self.is_empty_line().await? {
            self.parse_header().await?;
        }
        Ok(())
    }

    async fn is_empty_line(&mut self) -> Result<bool, HttpError> {
        self.ensure_buffer(2).await?;
        let is_empty = self.index + 1 < self.end && self.buffer[self.index] == CR && self.buffer[self.index + 1] == NL;
        return Ok(is_empty);
    }

    async fn parse_header(&mut self) -> Result<(HeaderName, HeaderValue), HttpError> {
        let header_name = self.parse_header_name().await?;
        self.eat(":").await?;
        let header_value = self.parse_header_value().await?;
        self.header_map.append(header_name.clone(), header_value.clone());
        self.parse_newline().await?;
        Ok((header_name, header_value))
    }

    async fn parse_header_name(&mut self) -> Result<HeaderName, HttpError> {
        let start = self.index;
        if !self.advance_until(':').await? {
            return Err(HttpError::ResponseParseErr);
        }
        let header_string = std::str::from_utf8(&self.buffer[start..self.index]);
        let header = header_string?.parse()?;

        Ok(header)
    }

    async fn parse_header_value(&mut self) -> Result<HeaderValue, HttpError> {
        let start = self.index;
        if !self.advance_until('\r').await? {
            return Err(HttpError::ResponseParseErr);
        }


        let header_value_string = std::str::from_utf8(&self.buffer[start..self.index])?.trim();

        Ok(header_value_string.parse()?)
    }

    async fn parse_body(&mut self) -> Result<Vec<u8>, HttpError> {
        let content_length: usize = match self.header_map.get("Content-Length").map(|h| h.to_str().unwrap()) {
            Some(c) => {
                c.trim().parse().unwrap()
            },
            None => {
                let mut existing_buffer: Vec<u8> = self
                    .buffer[self.index..self.end]
                    .iter()
                    .cloned()
                    .collect();
                self.clear_buffer();
                self.source.read_to_end(&mut existing_buffer).await?;
                return Ok(existing_buffer);
            }
        };
        if self.end - self.index <= content_length {
            let start = self.index;
            self.index += content_length;
            // the body can be fully read from the buffer
            Ok(self.buffer[start..(start + content_length)].iter().cloned().collect())
        } else {
            // body not fully in buffer, must be read in chunks
            let mut body: Vec<u8> = self.buffer[self.index..self.end]
                .iter()
                .cloned()
                .collect();

            self.clear_buffer();

            while body.len() < content_length {
                if self.at_end() {
                    return Err(HttpError::ResponseParseErr);
                }

                self.fill_buffer().await?;
                body.extend_from_slice(&self.buffer[self.index..self.end]);
                if self.index == self.end {
                    self.clear_buffer();
                }
            }

            Ok(body)
        }
    }

    async fn eat(&mut self, s: &str) -> Result<(), HttpError> {
        let bytes = s.as_bytes();
        self.ensure_buffer(bytes.len()).await?;
        for b in bytes {
            if self.index < self.end && self.buffer[self.index] != *b {
                return Err(HttpError::ResponseParseErr);
            }
            self.index += 1;
        }

        Ok(())
    }

    async fn advance_until(&mut self, c: char) -> Result<bool, HttpError> {
        while !self.at_end() { // && (self.index >= self.buffer.len() || self.buffer[self.index] != c as u8) {
            self.ensure_buffer(2).await?;

            if self.index < self.end && self.buffer[self.index] == c as u8 {
                return Ok(true);
            }

            self.index += 1;
        }

        Ok(self.index < self.end && self.buffer[self.index] == c as u8)
    }

    async fn fill_buffer(&mut self) -> Result<(), HttpError> {
        if self.at_end() {
            return Ok(());
        }
        let len = self.buffer.len();
        let bytes_read = self.source.read(&mut self.buffer[self.index..len]).await?;
        self.eof = bytes_read == 0;
        self.end += bytes_read;
        Ok(())
    }

    async fn refill_buffer(&mut self, min: usize) -> Result<(), HttpError> {
        if self.at_end() {
            return Ok(());
        }
        // fill up the buffer as much as we can!
        let mut new_buffer = vec![0u8; std::cmp::max(self.buffer.len(), min)];
        self.buffer.append(&mut new_buffer);
        let len = self.buffer.len();
        let bytes_read = self.source.read(&mut self.buffer[self.end..len]).await?;
        self.buffer.append(&mut new_buffer);

        self.eof = bytes_read == 0;
        self.end += bytes_read;

        Ok(())
    }

    async fn ensure_buffer(&mut self, min: usize) -> Result<(), HttpError> {
        if !self.at_end() && (self.end < self.index + min - 1) {
            self.refill_buffer(min).await?;
        }

        Ok(())
    }

    fn clear_buffer(&mut self) {
        self.index = 0;
        self.end = 0;
    }

    fn at_end(&self) -> bool {
        self.eof
    }
}*/

/*#[cfg(test)]
mod test {
    use actix_rt;
    use crate::http::HttpClient;
    use crate::http::HttpParser;
    use crate::http::HttpError;
    use http::HeaderMap;
    use http::Response;
    use http::header::HeaderValue;
    use http::status::StatusCode;
    use http::Request;

    #[actix_rt::test]
    async fn parses_very_simple_response() {
        let resp = parse_response("HTTP/1.1 200 OK\r\n\r\n").await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn parses_very_simple_response_tiny_buffer() {
        let resp = parse_response_with_options("HTTP/1.1 200 OK\r\n\r\n", 2usize).await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn parses_simple_response_two_headers() {
        let resp = parse_response("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: Close\r\n\r\nHi mum!").await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_header(resp.headers(), "Content-Type", "text/plain");
        assert_body(&resp, "Hi mum!");
    }

    #[actix_rt::test]
    async fn parses_simple_response_no_body() {
        let resp = parse_response("HTTP/1.1 301 OK\r\nLocation: http://website.thing\r\n\r\n").await;

        assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
        assert_header(resp.headers(), "Location", "http://website.thing");
        assert_eq!(resp.body().len(), 0);
    }

    #[actix_rt::test]
    async fn parses_simple_response_empty_headers() {
        let resp = parse_response("HTTP/1.1 200 OK\r\nEnjoy:\r\n\r\nHello there").await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_header(resp.headers(), "Enjoy", "");
        assert_body(&resp, "Hello there");
    }

    #[actix_rt::test]
    async fn parses_405_response_no_body() {
        let resp = parse_response("HTTP/1.1 405 Method Not Allowed\r\ncontent-length: 0\r\ndate: Mon, 15 Nov 2021 19:18:35 GMT\r\n\r\n").await;
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_header(resp.headers(), "Content-Length", "0");
    }

    #[actix_rt::test]
    async fn parses_very_simple_405_response() {
        let resp = parse_response("HTTP/1.1 405 Method Not Allowed\r\n\r\n").await;
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[actix_rt::test]
    async fn returns_error_for_truncated_response() {
        let err = HttpParser::new(&mut "HTTP/1.1 \n".as_bytes()).parse_response().await;
        assert_eq!(err.err().unwrap(), HttpError::ResponseParseErr);
    }

    #[actix_rt::test]
    async fn returns_error_for_invalid_header() {
        let err = HttpParser::new(&mut "HTTP/1.1 OK\r\nDoop\r\n\r\n".as_bytes()).parse_response().await;
        assert_eq!(err.err().unwrap(), HttpError::ResponseParseErr);
    }

    #[actix_rt::test]
    async fn do_google() {
        let resp = HttpClient::new(Request::builder()
                .uri("https://www.stuff.co.nz/")
                .body(String::new())
                .unwrap()
            )
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    async fn parse_response(text: &str) -> Response<Vec<u8>> {
        HttpParser::new(&mut text.as_bytes()).parse_response().await.unwrap()
    }

    async fn parse_response_with_options(text: &str, buffer_size: usize) -> Response<Vec<u8>> {
        HttpParser::with_options(&mut text.as_bytes(), buffer_size).parse_response().await.unwrap()
    }

    async fn assert_header(headers: &HeaderMap, key: &str, value: &str) {
        let actual_header: &HeaderValue = headers
            .get(key)
            .clone()
            .unwrap();
        let expected_header: HeaderValue = value.parse().unwrap();
        assert_eq!(actual_header, &expected_header);
    }

    async fn assert_body(resp: &Response<Vec<u8>>, body: &str) {
        assert_eq!(String::from_utf8(resp.body().clone()).unwrap(), body);
    }
}*/
