use actix_web::{HttpResponse, ResponseError};

#[derive(Fail, Debug)]
pub enum BlogError {
    #[fail(display = "Template Error: {:?}", _0)]
    TemplateError(String),

    #[fail(display = "Missing content: {:?}", _0)]
    MissingContent(String),

    #[fail(display = "I/O Error: {:?}", _0)]
    IOError(String)
}

impl ResponseError for BlogError {
    fn error_response(&self) -> HttpResponse {
        match self {
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}

impl From<tera::Error> for BlogError {
    fn from(e: tera::Error) -> Self {
        BlogError::TemplateError(format!("{}", e))
    }
}

impl From<std::io::Error> for BlogError {
    fn from(e: std::io::Error) -> Self {
        BlogError::IOError(format!("{}", e))
    }
}