use crate::html::{Node, NodeRef, Parser, Tokenizer};
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

/// Inline html resources into a single html buffer. Consumes input.
/// Media files are base64 encoded in data urls, text files are directly
/// embedded.
pub fn inline(mut input: String, base: &Path) -> Result<String, Box<dyn Error>> {
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
