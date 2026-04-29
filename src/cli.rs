#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub(crate) list: bool,
}
