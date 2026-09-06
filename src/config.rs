use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct Config {
    #[arg(short, long, default_value = "4001")]
    pub ib_port: u16,

    #[arg(short, long, default_value = "0")]
    pub client_id: u16,

    #[arg(short, long, default_value = "4045")]
    pub server_port: u16,
}
