use crate::DependencyTarget;
use crate::builder::*;

#[test]
fn test_call_builder_success() {
    let call = CallBuilder::new("/foo", "GET")
        .with_url("https://api.example.com")
        .with_content_type("application/json")
        .build()
        .expect("call should build successfully");

    assert_eq!(call.endpoint, "/foo");
    assert_eq!(call.url.as_deref(), Some("https://api.example.com"));
    assert_eq!(call.content_type.as_deref(), Some("application/json"));
    assert_eq!(call.method.to_string(), "GET");
}

#[test]
fn test_third_party_builder_builds() {
    let call = CallBuilder::new("/test", "POST").build().unwrap();

    let third_party = ThirdPartyBuilder::new("example", "http://localhost")
        .with_call("test-call", call)
        .build();

    assert!(third_party.is_ok());
}

#[test]
fn test_request_builder_with_dependency_and_body() {
    let dep = DependencyRequestBuilder::new("auth")
        .path_arg("token")
        .extractor("$.token", DependencyTarget::BearerAuthorization)
        .build();

    let req = RequestBuilder::new()
        .path_arg("endpoint")
        .query_arg("limit", "10")
        .header("Accept", "application/json")
        .with_dependency(dep)
        .body(serde_json::json!({"field": "value"}))
        .build();

    assert_eq!(req.path_arguments, vec!["endpoint"]);
    assert!(req.headers.contains_key("Accept"));
    assert_eq!(req.query_arguments["limit"], "10");
    assert!(req.body.is_some());
    assert!(req.dependency.is_some());
}

#[test]
fn test_dependency_request_extractors() {
    let req = DependencyRequestBuilder::new("auth")
        .extractor("$.token", DependencyTarget::Header("X-Auth-Token".into()))
        .extractor("$.user", DependencyTarget::BodyField("user_id".into()))
        .build();

    assert!(req.extractor.is_some());
    assert_eq!(req.extractor.as_ref().unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_with_query() {
    let call = CallBuilder::new("/get", "GET")
        .with_url("https://httpbin.org")
        .build()
        .unwrap();

    let tp = ThirdPartyBuilder::new("httpbin", "https://httpbin.org")
        .with_call("get", call)
        .build()
        .unwrap();

    let request = RequestBuilder::new().query_arg("foo", "bar").build();

    let response = tp.call("get", request).await.unwrap();
    let json: serde_json::Value = response.decode().unwrap();

    assert_eq!(json["args"]["foo"], "bar");
}

#[tokio::test]
async fn test_post_with_body() {
    let call = CallBuilder::new("/post", "POST")
        .with_url("https://httpbin.org")
        .build()
        .unwrap();

    let tp = ThirdPartyBuilder::new("httpbin", "https://httpbin.org")
        .with_call("post", call)
        .build()
        .unwrap();

    let request = RequestBuilder::new()
        .body(serde_json::json!({
            "name": "Rustacean",
            "lang": "Rust"
        }))
        .build();

    let response = tp.call("post", request).await.unwrap();
    let json: serde_json::Value = response.decode().unwrap();

    assert_eq!(json["json"]["name"], "Rustacean");
    assert_eq!(json["json"]["lang"], "Rust");
}

#[tokio::test]
async fn test_post_with_multipart_form() {
    let call = CallBuilder::new("/post", "POST")
        .with_url("https://httpbin.org")
        .build()
        .unwrap();

    let tp = ThirdPartyBuilder::new("httpbin", "https://httpbin.org")
        .with_call("post", call)
        .build()
        .unwrap();

    let form = http_client::Form::new()
        .text("name", "Rustacean")
        .text("lang", "Rust");

    let request = RequestBuilder::new().multipart(form).build();

    let response = tp.call("post", request).await.unwrap();
    let json: serde_json::Value = response.decode().unwrap();

    assert_eq!(json["form"]["name"], "Rustacean");
    assert_eq!(json["form"]["lang"], "Rust");
}

#[tokio::test]
async fn test_dependency_extraction() {
    // 1. First call: POST to /post to get a field back
    let dep_call = CallBuilder::new("/post", "POST")
        .with_url("https://httpbin.org")
        .build()
        .unwrap();

    // 2. Second call: GET with Authorization header set from dependency
    let get_call = CallBuilder::new("/get", "GET")
        .with_url("https://httpbin.org")
        .build()
        .unwrap();

    let tp = ThirdPartyBuilder::new("httpbin", "https://httpbin.org")
        .with_call("post", dep_call)
        .with_call("get", get_call)
        .build()
        .unwrap();

    let dep_req = DependencyRequestBuilder::new("post")
        .body(serde_json::json!({
            "token": "123456"
        }))
        .extractor("$.json.token", DependencyTarget::BearerAuthorization)
        .build();

    let req = RequestBuilder::new().with_dependency(dep_req).build();

    let response = tp.call("get", req).await.unwrap();
    let json: serde_json::Value = response.decode().unwrap();

    assert_eq!(json["headers"]["Authorization"], "Bearer 123456");
}
