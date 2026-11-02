use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "chinese", about = "A five-minute offline Mandarin lesson")]
struct Args {
    #[command(subcommand)]
    command: Option<chinese::cli::Command>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    chinese::cli::run(args.command)
}
