mod html;

use std::fs;
use std::io::prelude::*;

fn main() {
    let input_file = match std::env::args().nth(1).take() {
        Some(path) => path,
        None => {
            eprintln!("please provide path to input file");
            return;
        }
    };
    let mut input = String::new();
    match fs::File::open(input_file) {
        Ok(mut file) => {
            if let Err(err) = file.read_to_string(&mut input) {
                eprintln!("error: reading input file: {}", err);
                return;
            }
        }
        Err(err) => {
            eprintln!("error: opening input file: {}", err);
            return;
        }
    };
    let inlined_html = match html::inline(&input) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("error: inlining html: {}", err);
            return;
        }
    };
    if let Err(err) = std::io::stdout().write_all(inlined_html.as_bytes()) {
        eprintln!("error: writing to stdout: {}", err);
        return;
    };
}
