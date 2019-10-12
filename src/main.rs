mod html;
mod inline;

use clap::{App, Arg};
use inline::inline;
use std::fs;
use std::io::prelude::*;

fn main() {
    let cli = App::new("inliner")
        .author("Jack Mordaunt <jackmordaunt@gmail.com>")
        .about("Take html resources and bundle them into a single html file.")
        .arg(
            Arg::with_name("input")
                .required(true)
                .takes_value(true)
                .help("Path to html file"),
        )
        .arg(
            Arg::with_name("base")
                .required(false)
                .takes_value(true)
                .default_value(".")
                .help("Directory which links will be resolved against"),
        )
        .get_matches();
    let input = match fs::read_to_string(cli.value_of("input").unwrap()) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("error: opening input file: {}", err);
            return;
        }
    };
    let inlined = match inline(input, cli.value_of("base").unwrap().as_ref()) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: inlining html: {}", err);
            return;
        }
    };
    if let Err(err) = std::io::stdout().write_all(inlined.as_bytes()) {
        eprintln!("error: writing to stdout: {}", err);
        return;
    };
}
