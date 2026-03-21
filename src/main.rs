use std::{collections::HashMap, fmt::Write, fs::read_to_string, path::absolute};

use clap::Parser;
use osc8::Hyperlink;
use url::Url;
use yansi::Paint;

use crate::{cli::Cli, parse::parse_file, walk::build_walk_filtered};

mod cli;
mod parse;
mod walk;

fn main() {
    let cli = Cli::parse();
    // let db = FileDatabase::new("data");

    let mut collected = HashMap::new();

    for result in build_walk_filtered() {
        match result {
            Ok(entry) => {
                // only handle file
                let path = entry.path();
                let Ok(string) = read_to_string(path) else {
                    continue;
                };
                let new_items = parse_file(&cli, &string);
                collected.insert(path.to_path_buf(), new_items);
            }
            Err(err) => eprintln!("ERROR: {}", err),
        }
    }

    let mut outputs = String::new();

    let mut keys: Vec<_> = collected.keys().collect();
    keys.sort();

    for path in keys {
        let items = collected.get(path).unwrap();
        if items.is_empty() {
            continue;
        }
        let relative_path = path.strip_prefix(".").unwrap_or(path);
        let url = Url::from_file_path(absolute(path).unwrap()).unwrap();

        writeln!(
            outputs,
            "{url}{relative}{end}",
            url = Hyperlink::new(url.as_str()),
            relative = relative_path.display().bold(),
            end = Hyperlink::END
        )
        .unwrap();

        for item in items {
            writeln!(outputs, "  {}", item.trim()).unwrap();
        }
    }
    println!("{}", outputs);
}
