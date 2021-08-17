use std::{
    any::Any,
    convert::{TryFrom, TryInto},
};

use crate::{
    body::Body,
    error::{Error, ErrorBodyHasBeenTaken, Result},
    http::{
        header::{self, HeaderMap, HeaderName, HeaderValue},
        Extensions, Method, Uri, Version,
    },
    route_recognizer::Params,
};

/// Component parts of an HTTP Request
pub struct RequestParts {
    /// The request’s method
    method: Method,

    /// The request’s URI
    uri: Uri,

    /// The request’s version
    version: Version,

    /// The request’s headers
    headers: HeaderMap,

    /// The request’s extensions
    extensions: Extensions,
}

#[derive(Default)]
pub(crate) struct RequestState {
    pub(crate) match_params: Params,
}

/// Represents an HTTP request.
pub struct Request {
    method: Method,
    uri: Uri,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
    body: Option<Body>,
    pub(crate) state: RequestState,
}

impl Request {
    pub(crate) fn from_http_request(req: hyper::Request<hyper::Body>) -> Result<Self> {
        let (parts, body) = req.into_parts();
        Ok(Self {
            method: parts.method,
            uri: parts.uri,
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
            body: Some(Body(body)),
            state: Default::default(),
        })
    }

    /// Creates a request builder.
    pub fn builder() -> RequestBuilder {
        RequestBuilder(Ok(RequestParts {
            method: Method::GET,
            uri: Default::default(),
            version: Default::default(),
            headers: Default::default(),
            extensions: Default::default(),
        }))
    }

    /// Returns a reference to the associated HTTP method.
    #[inline]
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Sets the HTTP method for this request.
    #[inline]
    pub fn set_method(&mut self, method: Method) {
        self.method = method;
    }

    /// Returns a reference to the associated URI.
    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Sets the URI for this request.
    #[inline]
    pub fn set_uri(&mut self, uri: Uri) {
        self.uri = uri;
    }

    /// Returns the associated version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Sets the version for this request.
    #[inline]
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    /// Returns a reference to the associated header map.
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns a mutable reference to the associated header map.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Returns a reference to the associated extensions.
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    /// Returns a mutable reference to the associated extensions.
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    /// Sets the body for this request.
    pub fn set_body(&mut self, body: Body) {
        self.body = Some(body);
    }

    /// Take the body from this request and sets the body to empty.
    #[inline]
    #[must_use]
    pub fn take_body(&mut self) -> Result<Body> {
        self.body.take().ok_or(ErrorBodyHasBeenTaken.into())
    }
}

/// An request builder.
pub struct RequestBuilder(Result<RequestParts>);

impl RequestBuilder {
    /// Sets the HTTP method for this request.
    ///
    /// By default this is [`Method::GET`].
    #[must_use]
    pub fn method(self, method: Method) -> RequestBuilder {
        Self(self.0.map(move |parts| RequestParts { method, ..parts }))
    }

    /// Sets the URI for this request.
    ///
    /// By default this is `/`.
    #[must_use]
    pub fn uri<T>(self, uri: T) -> RequestBuilder
    where
        T: TryInto<Uri, Error = Error>,
    {
        Self(self.0.and_then(move |parts| {
            Ok(RequestParts {
                uri: uri.try_into()?,
                ..parts
            })
        }))
    }

    /// Sets the HTTP version for this request.
    #[must_use]
    pub fn version(self, version: Version) -> RequestBuilder {
        Self(self.0.map(move |parts| RequestParts { version, ..parts }))
    }

    /// Appends a header to this request builder.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal [HeaderMap] being constructed.
    #[must_use]
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<Error>,
    {
        Self(self.0.and_then(move |mut parts| {
            let key = <HeaderName as TryFrom<K>>::try_from(key).map_err(Into::into)?;
            let value = <HeaderValue as TryFrom<V>>::try_from(value).map_err(Into::into)?;
            parts.headers.append(key, value);
            Ok(parts)
        }))
    }

    /// Sets the `Content-Type` header on the request.
    #[must_use]
    pub fn content_type(self, content_type: &str) -> Self {
        Self(self.0.and_then(move |mut parts| {
            let value = content_type.parse()?;
            parts.headers.append(header::CONTENT_TYPE, value);
            Ok(parts)
        }))
    }

    /// Adds an extension to this request.
    #[must_use]
    pub fn extension<T>(self, extension: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        Self(self.0.map(move |mut parts| {
            parts.extensions.insert(extension);
            parts
        }))
    }

    /// Consumes this builder, using the provided body to return a constructed
    /// [Request].
    ///
    /// # Errors
    ///
    /// This function may return an error if any previously configured argument
    /// failed to parse or get converted to the internal representation. For
    /// example if an invalid `head` was specified via `header("Foo",
    /// "Bar\r\n")` the error will be returned when this function is called
    /// rather than when `header` was called.
    pub fn body(self, body: Body) -> Result<Request> {
        self.0.map(move |parts| Request {
            method: parts.method,
            uri: parts.uri,
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
            body: Some(body),
            state: Default::default(),
        })
    }
}
