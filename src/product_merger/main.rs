use std::io::Read;
use std::path::{Path, PathBuf};

use api::{Error, Result};
use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Debug, Clone)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Parser, Debug)]
#[command(name = "product-merger", version, about, long_about = None)]
struct Opts {
    #[arg(index = 1, help = "input products/base products json ('-' for stdin)", value_name = "INPUT_JSON")]
    input: PathBuf,

    #[arg(index = 2, help = "the destination of output ('-' or omitted for stdout)", value_name = "OUTPUT_JSON")]
    output: Option<PathBuf>,

    #[clap(
        short,
        long,
        default_value_t = false,
        help = "parse all the entries in the input JSON as base products"
    )]
    base_project_json: bool,

    #[clap(short, long, default_value = "info", help = "log level", value_name = "LEVEL")]
    level: Level,

    #[clap(short, long, help = "pretty print output")]
    pretty: bool,
}

impl Opts {
    pub fn verify(&self) -> Result<()> {
        let mut errs = Vec::new();
        if self.input != Path::new("-") && !self.input.exists() {
            errs.push(Error::FileNotFound(self.input.clone()));
        }
        if let Some(out) = &self.output
            && out != Path::new("-") && out.exists() {
                errs.push(Error::FileExists(out.clone()));
        }
        Error::from((), errs)
    }

    pub fn init(&self) -> Result<()> {
        unsafe {
            match self.level {
                Level::Error => std::env::set_var("RUST_LOG", "error"),
                Level::Warn => std::env::set_var("RUST_LOG", "warn"),
                Level::Info => std::env::set_var("RUST_LOG", "info"),
                Level::Debug => std::env::set_var("RUST_LOG", "debug"),
                Level::Trace => std::env::set_var("RUST_LOG", "trace"),
            }
        }
        env_logger::try_init().map_err(|e| Error::Fatal(e.to_string()))?;
        log::info!("set log level to {:?}", self.level);
        Ok(())
    }

    fn read_input(&self) -> Result<String> {
        if self.input == Path::new("-") {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(Error::Io)?;
            Ok(buf)
        } else {
            std::fs::read_to_string(&self.input).map_err(Error::Io)
        }
    }

    fn write_output(&self, json: &str) -> Result<()> {
        match &self.output {
            Some(path) if path != Path::new("-") => {
                std::fs::write(path, json).map_err(Error::Io)
            }
            _ => {
                println!("{}", json);
                Ok(())
            }
        }
    }
}

fn perform(opts: Opts) -> Result<()> {
    opts.verify()?;
    opts.init()?;

    let text = opts.read_input()?;
    let items = api::product::parse_items(&text, opts.base_project_json)?;
    let products = api::updater::update_all(items)?;
    let json = if opts.pretty {
        serde_json::to_string_pretty(&products)
    } else {
        serde_json::to_string(&products)
    }
    .map_err(Error::Parse)?;
    opts.write_output(&json)
}

fn main() {
    let opts = Opts::parse();
    if let Err(e) = perform(opts) {
        println!("Error: {}", e);
        std::process::exit(1);
    }
}
