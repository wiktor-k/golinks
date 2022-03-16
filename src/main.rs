use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use awc::{
    error::{JsonPayloadError, SendRequestError},
    Client,
};

use clap::Parser;
use serde::{Deserialize, Serialize};
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

async fn render(
    request: HttpRequest,
    slug: web::Path<String>,
    config: web::Data<Args>,
    client: Data<Client>,
) -> Result<HttpResponse, Error> {
    let links = client
        .get(&config.url)
        .insert_header((
            "User-Agent",
            "currencies (+https://metacode.biz/@wiktor#golinks3)",
        ))
        .insert_header(("Accept", "application/json"))
        .insert_header(("Cookie", format!("__Secure-Token={}", config.token)))
        .send()
        .await?
        .json::<Links>()
        .await?;

    let host = request
        .headers()
        .get("Host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let key = format!("{}/{}", host, slug);
    eprintln!("key = {}", key);

    if let Some(location) = &links.inner.get(&key) {
        Ok(HttpResponse::Found()
            .append_header(("Location", &location.href[..]))
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

#[derive(Parser, Debug, Clone)]
struct Args {
    #[clap(env = "URL")]
    url: String,

    #[clap(env = "TOKEN")]
    token: String,

    #[clap(env = "BIND", default_value = "127.0.0.1:8080")]
    bind: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let args = Args::parse();
    let bind = args.bind.clone();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(Client::default()))
            .app_data(Data::new(args.clone()))
            .route("/healthz", web::get().to(healthz))
            .route("/{slug}", web::get().to(render))
            .wrap(Logger::default())
    })
    .bind(bind)?
    .run()
    .await?;

    Ok(())
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
