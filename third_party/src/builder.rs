use std::collections::HashMap;

use serde::Serialize;

use crate::error::Error;
use crate::{Call, DependencyRequest, DependencyTarget, Request, ThirdParty};

pub struct ThirdPartyBuilder {
    name: String,
    base_url: String,
    calls: HashMap<String, Call>,
}

impl ThirdPartyBuilder {
    pub fn new(name: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into(),
            calls: HashMap::new(),
        }
    }

    pub fn with_call(mut self, name: impl Into<String>, call: Call) -> Self {
        self.calls.insert(name.into(), call);
        self
    }

    pub fn build(self) -> Result<ThirdParty, Error> {
        ThirdParty::new(&self.name, &self.base_url, self.calls)
    }
}

pub struct CallBuilder {
    endpoint: String,
    method: String,
    url: Option<String>,
    content_type: Option<String>,
}

impl CallBuilder {
    pub fn new(endpoint: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            method: method.into(),
            url: None,
            content_type: None,
        }
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    pub fn build(self) -> Result<Call, Error> {
        Ok(Call {
            endpoint: self.endpoint,
            method: self.method.as_str().try_into()?,
            url: self.url,
            content_type: self.content_type,
        })
    }
}

pub struct RequestBuilder<T = (), D = ()> {
    path_arguments: Vec<String>,
    query_arguments: HashMap<String, String>,
    body: Option<T>,
    headers: HashMap<String, String>,
    dependency: Option<DependencyRequest<D>>,
}

impl RequestBuilder<(), ()> {
    pub fn new() -> Self {
        Self {
            path_arguments: Vec::new(),
            query_arguments: HashMap::new(),
            body: None,
            headers: HashMap::new(),
            dependency: None,
        }
    }
}

impl<D> RequestBuilder<(), D> {
    pub fn body<T: Serialize>(self, value: T) -> RequestBuilder<T, D> {
        RequestBuilder {
            path_arguments: self.path_arguments,
            query_arguments: self.query_arguments,
            body: Some(value),
            headers: self.headers,
            dependency: self.dependency,
        }
    }
}

impl<T> RequestBuilder<T, ()> {
    pub fn with_dependency<D: Serialize>(self, dep: DependencyRequest<D>) -> RequestBuilder<T, D> {
        RequestBuilder {
            path_arguments: self.path_arguments,
            query_arguments: self.query_arguments,
            body: self.body,
            headers: self.headers,
            dependency: Some(dep),
        }
    }
}

impl<T, D> RequestBuilder<T, D> {
    pub fn path_arg(mut self, value: impl Into<String>) -> Self {
        self.path_arguments.push(value.into());
        self
    }

    pub fn query_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_arguments.insert(key.into(), value.into());
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> Request<T, D> {
        Request {
            path_arguments: self.path_arguments,
            query_arguments: self.query_arguments,
            body: self.body,
            headers: self.headers,
            dependency: self.dependency,
        }
    }
}

impl Default for RequestBuilder<(), ()> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DependencyRequestBuilder<T = ()> {
    name: String,
    path_arguments: Vec<String>,
    query_arguments: HashMap<String, String>,
    body: Option<T>,
    headers: HashMap<String, String>,
    extractor: Option<Vec<(String, DependencyTarget)>>,
}

impl DependencyRequestBuilder<()> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path_arguments: Vec::new(),
            query_arguments: HashMap::new(),
            body: None,
            headers: HashMap::new(),
            extractor: None,
        }
    }

    pub fn body<T: Serialize>(self, value: T) -> DependencyRequestBuilder<T> {
        DependencyRequestBuilder {
            name: self.name,
            path_arguments: self.path_arguments,
            query_arguments: self.query_arguments,
            body: Some(value),
            headers: self.headers,
            extractor: self.extractor,
        }
    }
}

impl<T> DependencyRequestBuilder<T>
where
    T: Serialize,
{
    pub fn path_arg(mut self, value: impl Into<String>) -> Self {
        self.path_arguments.push(value.into());
        self
    }

    pub fn query_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_arguments.insert(key.into(), value.into());
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn extractor(mut self, expr: impl Into<String>, target: DependencyTarget) -> Self {
        self.extractor.get_or_insert_with(Vec::new).push((expr.into(), target));
        self
    }

    pub fn extractors(
        mut self,
        list: impl IntoIterator<Item = (impl Into<String>, DependencyTarget)>,
    ) -> Self {
        let extractors = self.extractor.get_or_insert(Vec::new());
        extractors.extend(list.into_iter().map(|(e, t)| (e.into(), t)));
        self
    }

    pub fn build(self) -> DependencyRequest<T> {
        DependencyRequest {
            name: self.name,
            path_arguments: self.path_arguments,
            query_arguments: self.query_arguments,
            body: self.body,
            headers: self.headers,
            extractor: self.extractor,
        }
    }
}
