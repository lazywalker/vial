use {
    crate::{asset, util, Result},
    std::{
        collections::HashMap,
        error, fmt, fs,
        io::{self, BufReader},
    },
};

/// Each request ultimately ends in a `Response` that is served to the
/// client and then discarded, like fallen flower petals. Together
/// with [`Request`](struct.request.html) and
/// [`Responder`](trait.Responder.html) it forms the holy trinity of
/// `R`'s in Vial.
///
/// Rather than use the "Builder" pattern like more mature and better
/// designed libraries, Vial's `Response` lets you set properties
/// either directly or using Builder-style methods:
///
/// ```no_run
/// vial::routes! {
///     GET "/404" => |_| Response::from(404)
///         .with_header("Content-Type", "text/plain")
///         .with_body("404 Not Found");
/// }
/// ```
///
/// It also defaults to `text/html`, so you need to use
/// [`with_header()`](#method.with_header) or
/// [`header()`](#method.header) to send plain text.
pub struct Response {
    /// HTTP Status Code
    pub code: usize,
    /// The headers we're sending back.
    pub headers: HashMap<String, String>,
    /// Unclear why we need this...
    pub content_type: String,

    /// TODO: remove this
    pub body: String,
    /// TODO: remove this
    pub buf: Vec<u8>,
    /// TODO: only use this
    pub reader: Box<dyn io::Read>,
    /// TODO: hax
    pub is_reader: bool,
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("code", &self.code)
            .field("content_type", &self.content_type)
            .field("body", &self.body)
            .finish()
    }
}

impl Default for Response {
    fn default() -> Response {
        Response {
            code: 200,
            body: String::new(),
            buf: Vec::new(),
            headers: HashMap::new(),
            reader: Box::new(io::empty()),
            is_reader: false,
            content_type: "text/html; charset=utf8".to_string(),
        }
    }
}

impl Response {
    /// Create a new, empty, 200 response - ready for HTML!
    pub fn new() -> Response {
        Response::default()
    }

    /// Take a peek at all the headers for this response.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Get an individual header. `name` is case insensitive.
    pub fn header(&mut self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_lowercase())
    }

    /// Set an individual header.
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers
            .insert(name.to_string().to_lowercase(), value.to_string());
    }

    /// Convert into a Response.
    pub fn from<T: Into<Response>>(from: T) -> Response {
        from.into()
    }

    /// Create a response from an asset. See the
    /// [`asset`](asset/index.html) module for more information on
    /// using assets.
    pub fn from_asset(path: &str) -> Response {
        Response::default().with_asset(path)
    }

    /// Create a response from a (boxed) io::Read.
    pub fn from_reader(reader: Box<dyn io::Read>) -> Response {
        Response::default().with_reader(reader)
    }

    /// Creates a response from a file on disk.
    /// TODO: Path?
    pub fn from_file(path: &str) -> Response {
        Response::default().with_file(path)
    }

    /// Creates a 500 response from an error, displaying it.
    pub fn from_error<E: error::Error>(err: E) -> Response {
        Response::from(500).with_error(err)
    }

    /// Creates a new Response and sets the given header, in
    /// addition to the defaults.
    pub fn from_header(name: &str, value: &str) -> Response {
        Response::default().with_header(name, value)
    }

    /// Creates a new default Response with the given body.
    pub fn from_body<S: AsRef<str>>(body: S) -> Response {
        Response::default().with_body(body)
    }

    /// Creates a new `text/plain` Response with the given body.
    pub fn from_text<S: AsRef<str>>(text: S) -> Response {
        Response::default().with_text(text)
    }

    /// Creates a new response with the given HTTP Status Code.
    pub fn with_code(mut self, code: usize) -> Response {
        self.code = code;
        self
    }

    /// Body builder. Returns a Response with the given body.
    pub fn with_body<S: AsRef<str>>(mut self, body: S) -> Response {
        self.body.clear();
        self.body.push_str(body.as_ref());
        self
    }

    /// Returns a `text/plain` Response with the given body.
    pub fn with_text<S: AsRef<str>>(self, text: S) -> Response {
        self.with_body(text)
            .with_header("Content-Type", "text/plain")
    }

    /// Returns a Response using the given reader for the body.
    pub fn with_reader(mut self, reader: Box<dyn io::Read>) -> Response {
        self.reader = reader;
        self.is_reader = true;
        self
    }

    /// Uses an asset for the given body and sets the `Content-Type`
    /// header based on the file's extension.
    ///
    /// See the [`asset`](asset/index.html) module for more
    /// information on using assets.
    pub fn with_asset(mut self, path: &str) -> Response {
        if let Some(path) = asset::normalize_path(path) {
            if asset::exists(&path) {
                if asset::is_bundled() {
                    if let Some(reader) = asset::as_reader(&path) {
                        self.set_header("ETag", asset::etag(&path).as_ref());
                        self.content_type = util::content_type(&path).to_string();
                        return self.with_reader(reader);
                    }
                } else {
                    return self.with_file(&path);
                }
            }
        }
        self.with_code(404)
    }

    /// Sets this Response's body to the body of the given file and
    /// sets the `Content-Type` header based on the file's extension.
    pub fn with_file(mut self, path: &str) -> Response {
        match fs::File::open(path) {
            Ok(file) => {
                self.set_header("ETag", &asset::etag(path).as_ref());
                self.content_type.clear();
                self.content_type.push_str(util::content_type(path));
                self.with_reader(Box::new(BufReader::new(file)))
            }

            Err(e) => self.with_error(Box::new(e)),
        }
    }

    /// Sets the response code to 500 and the body to the error's text.
    pub fn with_error<E: error::Error>(self, err: E) -> Response {
        self.with_code(500)
            .with_body(&format!("<h1>500 Internal Error</h1><pre>{}", err))
    }

    /// Returns a Response with the given header set to the value.
    pub fn with_header(mut self, key: &str, value: &str) -> Response {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Length of the body.
    pub fn len(&self) -> usize {
        if self.is_reader {
            0
        } else if self.buf.is_empty() {
            self.body.len()
        } else {
            self.buf.len()
        }
    }

    /// Is ths response empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a 302 redirect to the given URL.
    pub fn redirect_to<U: AsRef<str>>(url: U) -> Response {
        Response::from(302).with_header("location", url.as_ref())
    }

    /// Writes this response to a stream.
    pub fn write<W: io::Write>(mut self, mut w: W) -> Result<()> {
        // we don't set Content-Length on static files we stream
        let content_length = if !self.is_empty() {
            format!("Content-Length: {}\r\n", self.len())
        } else {
            "".to_string()
        };

        // gross - move into print_headers or something
        let mut header = format!(
            "HTTP/1.1 {} OK\r\nServer: ~ vial {} ~\r\nDate: {}\r\nContent-Type: {}\r\n{}Connection: close\r\n",
            self.code, env!("CARGO_PKG_VERSION"), util::http_current_date(),
            self.content_type, content_length,
        );

        // TODO check for content-type, date, etc
        header.push_str(
            &self
                .headers
                .iter()
                .map(|(key, val)| format!("{}: {}", key, val))
                .collect::<Vec<_>>()
                .join("\r\n"),
        );

        if !header.ends_with("\r\n") {
            header.push_str("\r\n");
        }
        header.push_str("\r\n");

        w.write_all(header.as_bytes())?;

        if self.is_reader {
            io::copy(&mut self.reader, &mut w)?;
        } else if self.buf.is_empty() {
            w.write_all(self.body.as_bytes())?;
        } else {
            w.write_all(&self.buf)?;
        }

        w.flush()?;

        Ok(())
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.buf.is_empty() {
            write!(f, "{}", String::from_utf8_lossy(&self.buf))
        } else {
            write!(f, "{}", self.body)
        }
    }
}

impl From<&str> for Response {
    fn from(s: &str) -> Response {
        Response {
            body: s.to_string(),
            ..Response::default()
        }
    }
}

impl From<&String> for Response {
    fn from(s: &String) -> Response {
        Response {
            body: s.clone(),
            ..Response::default()
        }
    }
}

impl From<String> for Response {
    fn from(body: String) -> Response {
        Response {
            body,
            ..Response::default()
        }
    }
}

impl From<usize> for Response {
    fn from(i: usize) -> Response {
        Response {
            code: i,
            body: match i {
                404 => "404 Not Found".into(),
                500 => "500 Internal Server Error".into(),
                _ => "".into(),
            },
            ..Response::default()
        }
    }
}

impl From<std::borrow::Cow<'_, [u8]>> for Response {
    fn from(i: std::borrow::Cow<'_, [u8]>) -> Response {
        Response {
            body: String::from_utf8_lossy(&i).to_string(),
            ..Response::default()
        }
    }
}
