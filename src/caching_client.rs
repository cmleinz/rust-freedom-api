use std::sync::Arc;

use bytes::Bytes;
use freedom_config::Config;
use reqwest::{Response, StatusCode};
use url::Url;

use crate::{
    api::{Api, Container, Value},
    error::Error,
    Client,
};

/// An asynchronous `Client` for interfacing with the ATLAS freedom API, which implements query
/// caching.
///
/// This client has the same API as the normal [`Client`](crate::client::Client), however queries
/// and their associated responses are cached before being delivered.
///
/// As a result, the items which are returned to the caller are wrapped in [`Arc`](std::sync::Arc).
/// This makes cloning items out of the cache extremely cheap, regardless of the object's actual
/// size.
#[derive(Clone, Debug)]
pub struct CachingClient {
    pub(crate) inner: Client,
    pub(crate) cache: moka::future::Cache<Url, (Bytes, StatusCode)>,
}

impl PartialEq for CachingClient {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Value> Container<T> for Arc<T> {
    fn into_inner(self) -> T {
        std::sync::Arc::<T>::unwrap_or_clone(self)
    }
}

impl Api for CachingClient {
    type Container<T: Value> = Arc<T>;

    async fn delete(&self, url: Url) -> Result<Response, Error> {
        self.inner.delete(url).await
    }

    async fn get(&self, url: Url) -> Result<(Bytes, StatusCode), Error> {
        let client = &self.inner;
        let url_clone = url.clone();

        let fut = async {
            let (body, status) = client.get(url_clone).await?;

            Ok::<_, Error>((body, status))
        };

        let (body, status) = match self.cache.get(&url).await {
            Some(out) => out,
            None => fut.await?,
        };

        Ok((body, status))
    }

    async fn post<S>(&self, url: Url, msg: S) -> Result<Response, Error>
    where
        S: serde::Serialize + Send + Sync,
    {
        self.inner.post(url, msg).await
    }

    fn config(&self) -> &Config {
        self.inner.config()
    }

    fn config_mut(&mut self) -> &mut Config {
        self.inner.config_mut()
    }
}
