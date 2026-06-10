//! Handlers
//!
//! HTTP request handlers

use crate::error::AppResult;
use axum::{body::Body, extract::Request, response::Response};
use bytes::Bytes;

/// Handle incoming proxy requests
pub async fn handle_proxy_request(_request: Request) -> AppResult<Response> {
    // TODO: Implement request handling
    Ok(Response::builder().status(200).body(Body::empty()).unwrap())
}
