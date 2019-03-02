use libremexre::errors::log_err;
use luml::{SExpr, TopLevel};
use std::{error::Error, fs::read_to_string, path::PathBuf, process::exit};
use structopt::StructOpt;

fn main() {
    let options = Options::from_args();
    options.start_logger();
    if let Err(err) = run(options) {
        log_err(&*err);
        exit(1);
    }
}

fn run(options: Options) -> Result<(), Box<dyn Error>> {
    let input = read_to_string(options.input_file)?;
    let sexprs = SExpr::parse_many(&input)?;

    let toplevels = sexprs
        .into_iter()
        .map(TopLevel::from_sexpr)
        .collect::<Result<Vec<_>, _>>()?;

    let interfaces = toplevels
        .iter()
        .filter(|tl| match tl {
            TopLevel::Interface { .. } => true,
            _ => false,
        })
        .map(|tl| tl.name())
        .collect::<Vec<_>>();

    println!("digraph {{");
    for tl in &toplevels {
        println!(
            "\t{} [label={:?},shape=\"record\"];",
            tl.name(),
            tl.uml_node()
        );
        for s in tl.uml_edges(&interfaces) {
            println!("{}", s);
        }
    }
    println!("}}");

    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "::structopt::clap::AppSettings::ColoredHelp"))]
pub struct Options {
    /// Turns off message output.
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,

    /// Increases the verbosity. Default verbosity is warnings and higher to syslog, info and
    /// higher to the console.
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,

    /// The input file.
    #[structopt(parse(from_os_str))]
    pub input_file: PathBuf,
}

impl Options {
    fn start_logger(&self) {
        stderrlog::new()
            .quiet(self.quiet)
            .timestamp(stderrlog::Timestamp::Second)
            .verbosity(self.verbose + 1)
            .init()
            .unwrap()
    }
}
