use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(env, help = "Server default URL")]
    pub server_default: String,
    #[arg(env, help = "Server fallback URL")]
    pub server_fallback: String,
    #[arg(short, long, env, help = "Run as leader node")]
    pub leader: bool,
    #[arg(short, long, env, default_value_t = 9999, help = "Port to run the server on")]
    pub port: u16,
}

