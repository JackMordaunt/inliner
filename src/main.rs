mod html;

use html::{Node, NodeRef, Parser, Tokenizer};
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
    let base_dir = match std::env::args().nth(2).take() {
        Some(path) => path,
        None => ".".to_string(),
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
            // Todo: embed manifest file with data url.
            if attr
                .get("rel")
                .and_then(|rel| Some(rel == "manifest"))
                .unwrap_or(false)
            {
                return Ok(());
            }
            if let Some(link) = attr.get("href").or(attr.get("src")) {
                use std::cell::RefCell;
                use std::path::PathBuf;
                use std::rc::Rc;
                let link = link.trim_matches('/');
                // Todo: Embed images via base64 encoded data urls.
                if link.ends_with("png") || link.ends_with("ico") || link.ends_with("jpeg") {
                    return Ok(());
                }
                let path = PathBuf::from(base_dir.clone()).join(link);
                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("{}: {:?}", &path.to_string_lossy(), e))?;
                if link.ends_with("css") {
                    *name = "style".to_string();
                    attr.remove("rel");
                }
                attr.remove("href");
                attr.remove("src");
                children.clear();
                children.push(Rc::new(RefCell::new(Node::Text(content))));
            }
        }
        Ok(())
    })
    .expect("inlining files");
    if let Err(err) = std::io::stdout().write_all(dom.to_string().as_bytes()) {
        eprintln!("error: writing to stdout: {}", err);
        return;
    };
}
