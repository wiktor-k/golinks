use golinks::start;
use std::net::TcpListener;

#[actix_rt::test]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = format!("http://127.0.0.1:{}/healthz", listener.local_addr()?.port());
    let _ = tokio::spawn(start(("".into(), "".into()), listener.into())?);
    let client = awc::Client::default();
    let response = client.get(address).send().await.unwrap();

    assert_eq!(200, response.status().as_u16());

    Ok(())
}
