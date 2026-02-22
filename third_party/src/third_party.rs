use std::collections::HashMap;
use std::sync::Arc;

use http_client::{Body, Client, Form, Method, Options, Request as HttpRequest};
use logger::Logger;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json_path::JsonPath;

use crate::error::Error;

pub struct ThirdParty {
    calls: HashMap<String, Call>,
    client: Client,
    logger: Arc<Logger>,
}

impl ThirdParty {
    pub(crate) fn new(
        name: &str,
        base_url: &str,
        calls: HashMap<String, Call>,
    ) -> Result<Self, Error> {
        let client = Client::new(Options {
            base_url: base_url.to_string(),
            content_type: None,
            timeout: None,
        })?;

        Ok(Self {
            calls,
            client,
            logger: Arc::new(Logger::new(format!("third_party:{}", name).as_str())),
        })
    }

    pub async fn call<T: Serialize, D: Serialize>(
        &self,
        call: &str,
        request: Request<T, D>,
    ) -> Result<Response, Error> {
        self.logger.debug(
            "starting request",
            Some(serde_json::json!({
                "call": call,
            })),
        );

        let call = self
            .calls
            .get(call)
            .ok_or(Error::CallNotFound(call.to_string()))?;

        // If no dependency
        if request.dependency.is_none() {
            return call.execute(&self.client, request).await;
        }

        if request.multipart.is_some() {
            // Multipart flow: dependency can only target header/query/path/bearer.
            let mut final_request = request;

            if let Some(mut dependency) = final_request.dependency.take() {
                let extractor = dependency.take_extractor();
                let result = self.execute_dependency(dependency).await?;
                final_request.apply_dependency_target_without_body(result, extractor)?;
            }

            let result = call.execute(&self.client, final_request).await;
            self.logger.debug("request finished", None);
            return result;
        }

        // JSON flow
        let mut final_request = request.try_into_json()?;
        if let Some(mut dependency) = final_request.dependency.take() {
            let extractor = dependency.take_extractor();
            let result = self.execute_dependency(dependency).await?;
            final_request.apply_dependency_target(result, extractor)?;
        }
        let result = call.execute(&self.client, final_request).await;
        self.logger.debug("request finished", None);
        result
    }

    async fn execute_dependency<T: Serialize>(
        &self,
        dependency: DependencyRequest<T>,
    ) -> Result<Response, Error> {
        let call = self
            .calls
            .get(dependency.name.as_str())
            .ok_or(Error::CallNotFound(dependency.name.clone()))?;

        call.execute(&self.client, dependency.into()).await
    }
}

pub struct Call {
    pub(crate) endpoint: String,
    pub(crate) method: Method,
    pub(crate) url: Option<String>,
    pub(crate) content_type: Option<String>,
}

impl Call {
    pub(crate) async fn execute<T: Serialize, D>(
        &self,
        client: &Client,
        request: Request<T, D>,
    ) -> Result<Response, Error> {
        let client_request = HttpRequest {
            url: request.url(self.url.as_deref(), &self.endpoint),
            method: self.method.clone(),
            content_type: self.content_type.clone(),
            headers: Some(request.headers),
            body: match (request.body, request.multipart) {
                (Some(body), None) => Some(Body::Json(serde_json::to_value(body)?)),
                (None, Some(form)) => Some(Body::Multipart(form)),
                (None, None) => None,
                (Some(_), Some(_)) => return Err(Error::CannotUseJsonAndMultipartTogether),
            },
        };

        let client_response = client.send_request(client_request).await?;
        Ok(client_response.into())
    }
}

pub struct Request<T = (), D = ()> {
    pub(crate) path_arguments: Vec<String>,
    pub(crate) query_arguments: HashMap<String, String>,
    pub(crate) body: Option<T>,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) multipart: Option<Form>,
    pub(crate) dependency: Option<DependencyRequest<D>>,
}

impl Request<serde_json::Value, serde_json::Value> {
    pub(crate) fn apply_dependency_target(
        &mut self,
        response: Response,
        extractors: Option<Vec<(String, DependencyTarget)>>,
    ) -> Result<(), Error> {
        if let Some(extractors) = extractors {
            let body = std::str::from_utf8(&response.body)?;
            let json_body: serde_json::Value = serde_json::from_str(body)?;

            // All extracted values of body kind will be put in a single JSON
            // body.
            let mut response_body = serde_json::Map::new();

            for (extractor, target) in extractors {
                let value = self.extract_value(&json_body, &extractor)?;

                if let DependencyTarget::BodyField(field) = target {
                    response_body.insert(field, serde_json::Value::String(value));
                } else {
                    self.apply_to_target(target, value, &mut HashMap::new());
                }
            }

            if let Some(mut existing_body) = self.body.take() {
                if let Some(existing_map) = existing_body.as_object_mut() {
                    existing_map.extend(response_body);
                    self.body = Some(serde_json::Value::Object(existing_map.clone()));
                } else {
                    return Err(Error::CannotHandleBodyWithDependencyBody);
                }
            } else {
                self.body = Some(serde_json::Value::Object(response_body));
            }
        }

        Ok(())
    }

    fn extract_value(&self, json: &serde_json::Value, extractor: &str) -> Result<String, Error> {
        let path = JsonPath::parse(extractor)?;

        match path.query(json).first() {
            None => Err(Error::ExtractorValueNotFound(extractor.to_string())),
            Some(value) => {
                let extracted = if let serde_json::Value::String(s) = value {
                    s.clone()
                } else {
                    serde_json::to_string(value)?
                };

                Ok(extracted)
            }
        }
    }

    fn apply_to_target(
        &mut self,
        target: DependencyTarget,
        value: String,
        body: &mut HashMap<String, String>,
    ) {
        match target {
            DependencyTarget::Header(key) => {
                self.headers.insert(key, value);
            }
            DependencyTarget::QueryParam(param) => {
                self.query_arguments.insert(param, value);
            }
            DependencyTarget::PathParam => {
                self.path_arguments.push(value);
            }
            DependencyTarget::BodyField(field) => {
                body.insert(field, value);
            }
            DependencyTarget::BearerAuthorization => {
                self.headers
                    .insert("Authorization".to_string(), format!("Bearer {value}"));
            }
        }
    }
}

impl<T, D> Request<T, D> {
    pub(crate) fn apply_dependency_target_without_body(
        &mut self,
        response: Response,
        extractors: Option<Vec<(String, DependencyTarget)>>,
    ) -> Result<(), Error> {
        if let Some(extractors) = extractors {
            let body = std::str::from_utf8(&response.body)?;
            let json_body: serde_json::Value = serde_json::from_str(body)?;

            for (extractor, target) in extractors {
                let value = extract_value(&json_body, &extractor)?;
                match target {
                    DependencyTarget::Header(key) => {
                        self.headers.insert(key, value);
                    }
                    DependencyTarget::QueryParam(param) => {
                        self.query_arguments.insert(param, value);
                    }
                    DependencyTarget::PathParam => {
                        self.path_arguments.push(value);
                    }
                    DependencyTarget::BearerAuthorization => {
                        self.headers
                            .insert("Authorization".to_string(), format!("Bearer {value}"));
                    }
                    DependencyTarget::BodyField(_) => {
                        return Err(Error::CannotApplyDependencyBodyFieldToMultipart);
                    }
                }
            }
        }

        Ok(())
    }
}

fn extract_value(json: &serde_json::Value, extractor: &str) -> Result<String, Error> {
    let path = JsonPath::parse(extractor)?;
    match path.query(json).first() {
        None => Err(Error::ExtractorValueNotFound(extractor.to_string())),
        Some(value) => {
            if let serde_json::Value::String(s) = value {
                Ok(s.clone())
            } else {
                Ok(serde_json::to_string(value)?)
            }
        }
    }
}

impl<T, D> Request<T, D> {
    pub(crate) fn url(&self, base_url: Option<&str>, endpoint: &str) -> String {
        let base_path = base_url.unwrap_or("");
        let path = self.build_path(endpoint);
        let query = self.build_query_string();

        match (base_path.is_empty(), query.is_empty()) {
            (true, true) => path,
            (true, false) => format!("{path}?{query}"),
            (false, true) => format!("{}{}", base_path.trim_end_matches('/'), path),
            (false, false) => format!("{}{}?{}", base_path.trim_end_matches('/'), path, query),
        }
    }

    fn build_path(&self, endpoint: &str) -> String {
        let base_path = if endpoint.starts_with('/') {
            endpoint.to_string()
        } else {
            format!("/{endpoint}")
        };

        self.path_arguments.iter().fold(base_path, |mut acc, arg| {
            acc.push('/');
            acc.push_str(arg);
            acc
        })
    }

    fn build_query_string(&self) -> String {
        if self.query_arguments.is_empty() {
            return String::new();
        }

        self.query_arguments
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }
}

impl<T, D> Request<T, D>
where
    T: Serialize,
    D: Serialize,
{
    pub(crate) fn try_into_json(
        self,
    ) -> Result<Request<serde_json::Value, serde_json::Value>, Error> {
        if self.multipart.is_some() {
            return Err(Error::CannotConvertMultipartToJsonRequest);
        }

        let json_body = match self.body {
            Some(b) => Some(serde_json::to_value(b)?),
            None => None,
        };

        let json_dep: Option<DependencyRequest<serde_json::Value>> = match self.dependency {
            Some(dep) => {
                let body = dep.body.map(serde_json::to_value).transpose()?;

                Some(DependencyRequest {
                    name: dep.name,
                    path_arguments: dep.path_arguments,
                    query_arguments: dep.query_arguments,
                    body,
                    headers: dep.headers,
                    extractor: dep.extractor,
                })
            }
            None => None,
        };

        Ok(Request {
            path_arguments: self.path_arguments,
            query_arguments: self.query_arguments,
            body: json_body,
            multipart: None,
            headers: self.headers,
            dependency: json_dep,
        })
    }
}

impl<T: Clone> From<&DependencyRequest<T>> for Request<T> {
    fn from(request: &DependencyRequest<T>) -> Self {
        Self {
            path_arguments: request.path_arguments.clone(),
            query_arguments: request.query_arguments.clone(),
            body: request.body.clone(),
            multipart: None,
            headers: request.headers.clone(),
            dependency: None,
        }
    }
}

impl<T> From<DependencyRequest<T>> for Request<T> {
    fn from(request: DependencyRequest<T>) -> Self {
        Self {
            path_arguments: request.path_arguments,
            query_arguments: request.query_arguments,
            body: request.body,
            multipart: None,
            headers: request.headers,
            dependency: None,
        }
    }
}

#[derive(Clone)]
pub struct DependencyRequest<T = ()> {
    pub(crate) name: String,
    pub(crate) path_arguments: Vec<String>,
    pub(crate) query_arguments: HashMap<String, String>,
    pub(crate) body: Option<T>,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) extractor: Option<Vec<(String, DependencyTarget)>>,
}

impl<T> DependencyRequest<T> {
    pub(crate) fn take_extractor(&mut self) -> Option<Vec<(String, DependencyTarget)>> {
        self.extractor.take()
    }
}

#[derive(Clone)]
pub enum DependencyTarget {
    Header(String),
    QueryParam(String),
    PathParam,
    BodyField(String),
    BearerAuthorization,
}

#[derive(Debug)]
pub struct Response {
    pub status_code: u16,
    pub headers: HashMap<String, Vec<u8>>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, Error> {
        Ok(serde_json::from_slice(&self.body)?)
    }
}

impl From<http_client::Response> for Response {
    fn from(response: http_client::Response) -> Self {
        Self {
            status_code: response.status_code,
            headers: response.headers,
            body: response.body,
        }
    }
}
