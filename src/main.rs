use clap::Parser;

#[derive(Parser, Debug)]
#[command(author,version,about,long_about=None)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

fn main() -> youtui::Result<()> {
    let args = Args::parse();
    match args {
        _ => youtui::run_app(),
    }
}
