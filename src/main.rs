use clap::Parser;
use golinks::start;
use service_binding::Binding;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    #[clap(env = "URL")]
    url: String,

    #[clap(env = "TOKEN")]
    token: String,

    #[clap(env = "TOKEN_FILE")]
    token_file: Option<PathBuf>,

    #[clap(env = "HOST", default_value = "tcp://127.0.0.1:8080")]
    bind: Binding,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Args::parse();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    if let Some(token_file) = &args.token_file {
        args.token = {
            use std::io::prelude::*;
            let mut f = std::fs::File::open(&token_file)?;
            let mut buffer = vec![];
            f.read_to_end(&mut buffer)?;
            String::from_utf8_lossy(&buffer).to_string()
        };
    }
    let listener = args.bind.try_into()?;
    let args = (args.url, args.token);

    start(args, listener)?.await?;
    Ok(())
}
