mod html;

use clap::{App, Arg};
use html::{Node, NodeRef, Parser, Tokenizer};
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

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

// Inline html resources into a single html buffer. Consumes input.
// Media files are base64 encoded in data urls, text files are directly
// embedded.
fn inline(mut input: String, base: &Path) -> Result<String, Box<dyn Error>> {
    let dom = Parser::new(Tokenizer::new(input.drain(..)).merged())
        .parse()
        .expect("parsing dom");
    dom.depth_first(&|n: NodeRef| {
        if let Node::Tag {
            name,
            attributes,
            children,
        } = &mut *n.borrow_mut()
        {
            let attr = attributes;
            if let Some(link) = attr.get("href").or(attr.get("src")) {
                let link = link.trim_matches('/');
                let path = base.join(link);
                let is_plain_text = ["html", "js", "css"].into_iter().fold(false, |acc, ext| {
                    if acc {
                        true
                    } else {
                        link.ends_with(ext)
                    }
                });
                match is_plain_text {
                    false => {
                        let file = fs::File::open(&path)
                            .map_err(|e| format!("{}: {:?}", &path.to_string_lossy(), e))?;
                        let content = base64::encode(
                            BufReader::new(file)
                                .bytes()
                                .map(Result::ok)
                                .filter_map(|b| b)
                                .collect::<Vec<u8>>()
                                .as_slice(),
                        );
                        let data_url = format!(
                            "data:{media_type};bas64,{data}",
                            media_type = mime_guess::from_path(&link).first_or_octet_stream(),
                            data = content
                        );
                        if attr.contains_key("href") {
                            attr.insert("href".into(), data_url);
                        } else if attr.contains_key("src") {
                            attr.insert("src".into(), data_url);
                        }
                    }
                    true => {
                        let content = fs::read_to_string(&path)
                            .map_err(|e| format!("{}: {:?}", &path.to_string_lossy(), e))?;
                        if link.ends_with("css") {
                            *name = "style".to_string();
                            attr.remove("rel");
                        }
                        attr.remove("href");
                        attr.remove("src");
                        children.clear();
                        children.push(Node::Text(content).into());
                    }
                };
            }
        }
        Ok(())
    })?;
    Ok(dom.to_string())
}
