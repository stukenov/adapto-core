use clap::Parser;

fn main() {
    let cli = adapto_cli::commands::Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(e) = adapto_cli::commands::run(cli) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
