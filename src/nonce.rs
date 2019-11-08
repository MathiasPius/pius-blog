use std::rc::Rc;
use std::fmt::{Formatter, Display};
use actix_service::{Service, Transform};
use actix_web::{
    http::header::{HeaderValue, CONTENT_SECURITY_POLICY},
    dev::{ServiceRequest, ServiceResponse, Extensions, Payload},
    FromRequest,
    HttpRequest, Error, HttpMessage
};
use futures::future::{ok, FutureResult};
use futures::{Future, Poll};
use rand::Rng;

#[derive(Clone)]
pub struct NonceInner(String);
impl Default for NonceInner {
    fn default() -> Self {
        let nonce: [u8; 32] = rand::thread_rng().gen();
        NonceInner(base64::encode(&nonce))
    }
}

pub struct Nonce(Rc<NonceInner>);

impl serde::Serialize for Nonce {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        serializer.serialize_str(&(*self.0).0)
    }
}

impl Display for Nonce {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &(*self.0).0)
    }
}

pub trait NonceRetrieval {
    fn get_nonce(&self) -> Nonce;
}

impl NonceRetrieval for HttpRequest {
    fn get_nonce(&self) -> Nonce {
        Nonce::get_nonce(&self.extensions()).unwrap()
    }
}

impl NonceRetrieval for ServiceRequest {
    fn get_nonce(&self) -> Nonce {
        Nonce::get_nonce(&self.extensions()).unwrap()
    }
}

impl FromRequest for Nonce {
    type Error = Error;
    type Future = Result<Self, Self::Error>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        Ok(req.get_nonce())
    }
}

impl Nonce {
    fn get_nonce(extensions: &Extensions) -> Option<Nonce> {
        if let Some(s_impl) = extensions.get::<Rc<NonceInner>>() {
            return Some(Nonce(Rc::clone(&s_impl)));
        }
        None
    }
}


#[derive(Default)]
pub struct CSPNonce;

impl<S, B> Transform<S> for CSPNonce 
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = CSPNonceMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(CSPNonceMiddleware { service })
    }
}

pub struct CSPNonceMiddleware<S> {
    service: S
}

impl<S, B> Service for CSPNonceMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let nonce = NonceInner::default();
        req.extensions_mut().insert(Rc::from(nonce.clone()));

        let policy = format!("default-src: {default}; script-src: {script}; style-src: {style}, connect-src: {connect}",
            default = "'self'",
            script = format!("nonce-{}", &nonce.0),
            style = format!("'unsafe-inline' nonce-{}", &nonce.0),
            connect = "'self' ws: wss:;"
        );

        Box::new(self.service.call(req).and_then(move |mut res| {
            if !res.headers().contains_key(&CONTENT_SECURITY_POLICY) {
                res.headers_mut().insert(
                    CONTENT_SECURITY_POLICY,
                    HeaderValue::from_str(&policy).unwrap(),
                );
            }
            Ok(res)
        }))
    }
}