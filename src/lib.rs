use actix_web::{
    dev::Server,
    middleware::Logger,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use awc::{
    error::{JsonPayloadError, SendRequestError},
    Client,
};

use serde::{Deserialize, Serialize};
use service_binding::Listener;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
struct Link {
    href: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Links {
    content: String,
    order: f64,
    #[serde(flatten)]
    inner: HashMap<String, Link>,
}

#[derive(Debug)]
struct Error;

impl actix_web::error::ResponseError for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<SendRequestError> for Error {
    fn from(_: SendRequestError) -> Self {
        Self
    }
}

impl From<JsonPayloadError> for Error {
    fn from(_: JsonPayloadError) -> Self {
        Self
    }
}

impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self
    }
}

#[derive(Debug, Serialize)]
struct NotFound {
    key: String,
}

async fn render_path(
    request: HttpRequest,
    slug: web::Path<String>,
    config: web::Data<(String, String)>,
    client: Data<Client>,
) -> Result<HttpResponse, Error> {
    let host = request
        .headers()
        .get("Host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let key = format!("{}/{}", host, slug);

    render_for_uri(config, client, key).await
}

async fn render_query(
    request: HttpRequest,
    config: web::Data<(String, String)>,
    client: Data<Client>,
) -> Result<HttpResponse, Error> {
    let key = request.query_string();
    render_for_uri(config, client, key.into()).await
}

async fn render_for_uri(
    config: web::Data<(String, String)>,
    client: Data<Client>,
    key: String,
) -> Result<HttpResponse, Error> {
    let links = client
        .get(&config.0)
        .insert_header((
            "User-Agent",
            "golinks (+https://metacode.biz/@wiktor#golinks3)",
        ))
        .insert_header(("Accept", "application/json"))
        .insert_header(("Cookie", format!("__Secure-Token={}", config.1)))
        .send()
        .await?
        .json::<Links>()
        .await?;

    if let Some(location) = &links.inner.get(&key) {
        Ok(HttpResponse::Found()
            .append_header(("Location", &location.href[..]))
            .append_header(("Access-Control-Allow-Origin", "*"))
            .append_header(("X-Collation-Id", uuid::Uuid::new_v4().to_string()))
            .body(""))
    } else {
        Ok(HttpResponse::NotFound()
            .append_header(("Content-Type", "application/problem+json"))
            .body(serde_json::to_string(&NotFound { key })?))
    }
}

async fn healthz() -> impl Responder {
    "OK"
}

pub fn start(args: (String, String), listener: Listener) -> std::io::Result<Server> {
    let server = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(Client::default()))
            .app_data(Data::new(args.clone()))
            .route("/", web::get().to(render_query))
            .route("/healthz", web::get().to(healthz))
            .route("/{slug}", web::get().to(render_path))
            .wrap(Logger::default())
    });

    let server = match listener {
        Listener::Tcp(listener) => server.listen(listener)?,
        Listener::Unix(listener) => server.listen_uds(listener)?,
    };

    Ok(server.run())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fransform() {
        let json = r#"{
  "content": "Quick links\nUse https://go.metacode.biz/X where X is a link code from below.",
  "order": 1499010613401.3,
  "go.metacode.biz/1": {
    "href": "https://metacode.biz"
  },
  "go.metacode.biz/tlsc": {
    "href": "https://www.ssllabs.com/ssltest/viewMyClient.html"
  }}"#;
        let links: Links = serde_json::from_str(json).unwrap();
        assert_eq!(links.inner.len(), 2);
        assert_eq!(
            links.inner["go.metacode.biz/1"],
            Link {
                href: "https://metacode.biz".into()
            }
        );
    }
}
