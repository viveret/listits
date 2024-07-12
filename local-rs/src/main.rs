use actix_cors::Cors;
use actix_service::{Service, Transform};
use actix_web::{
    body::BoxBody, dev::{ServiceRequest, ServiceResponse}, error::Error, http::header, middleware::Logger, web, App, HttpResponse, HttpServer
};
use futures_util::future::{ok, Ready};
use std::{fmt::Display, task::{Context, Poll}};

// Define handler functions for different routes
async fn index() -> impl actix_web::Responder {
    "Hello, world!"
}

async fn hello() -> impl actix_web::Responder {
    "Hello, from the /hello endpoint!"
}

async fn goodbye() -> impl actix_web::Responder {
    "Goodbye, from the /goodbye endpoint!"
}

async fn simulateError() -> impl actix_web::Responder {
    // Simulate an error by dividing by zero
    let _ = 1 / 0;
    "This will never be returned"
}

// Custom middleware for handling 500 Internal Server Error
pub struct ErrorHandler;

impl actix_service::Transform<ServiceRequest, ServiceResponse<BoxBody>> for ErrorHandler {
    type Error = Error;
    type InitError = ();
    type Response = ServiceResponse<BoxBody>;
    type Transform = ErrorHandlerMiddleware;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, _: ServiceRequest) -> Self::Future {
        ok(ErrorHandlerMiddleware)
    }
}

pub struct ErrorHandlerMiddleware;

impl actix_service::Service<ServiceRequest> for ErrorHandlerMiddleware {
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = req.into_response(HttpResponse::InternalServerError()
            .content_type("text/html")
            .body("<html><body><h1>Internal Server Error</h1><p>Something went wrong.</p></body></html>")
        );

        Box::pin(async { Ok(fut) })
    }
}

impl actix_service::Service<ServiceResponse> for ErrorHandlerMiddleware {
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, req: ServiceResponse) -> Self::Future {
        panic!("Error handling middleware called unexpectedly")
    }
}

pub struct ResponseInterceptor;

impl<S, B> Transform<S, ServiceRequest> for ResponseInterceptor
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: std::fmt::Debug + 'static,
{
    type Response = ServiceResponse<actix_web::web::Bytes>;
    type Error = Error;
    type InitError = ();
    type Transform = ResponseInterceptorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ResponseInterceptorMiddleware { service })
    }
}

pub struct ResponseInterceptorMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ResponseInterceptorMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: std::fmt::Debug + 'static,
{
    type Response = ServiceResponse<actix_web::web::Bytes>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            // Intercept and modify response here
            let modified_response = res.map_body(|head, b| {
                // Modify the body if needed
                actix_web::web::Bytes::from(format!("Intercepted: {:?}", b))
            });

            Ok(modified_response)
        })
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Start HTTP server and bind to port 8080
    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH", "HEAD"])
            .allowed_headers(vec![header::CONTENT_TYPE])
            .max_age(3600);

        App::new()
            // Add CORS middleware
            .wrap(cors)
            // Add response interception middleware
            .wrap(ResponseInterceptor)
            // Add request logging middleware
            .wrap(Logger::default())
            // Add error handling middleware
            .app_data(ErrorHandler)
            // Define routes and associate with handler functions
            .service(web::resource("/").to(index))
            .service(web::resource("/hello").to(hello))
            .service(web::resource("/goodbye").to(goodbye))
            .service(web::resource("/error").to(simulateError))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
