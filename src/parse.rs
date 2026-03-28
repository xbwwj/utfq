use chrono::NaiveDate;

use crate::cli::Cli;

/// 解析文件，同时生成链接。
pub fn parse_file(cli: &Cli, date: NaiveDate, text: &str) -> Vec<(bool, String)> {
    let mut items = vec![];
    let agmd_str = format!("<agmd:{}>", date);

    if !text.contains("<agmd:") {
        return items;
    }

    for line in text.lines() {
        if line.starts_with("<!--") {
            continue;
        }
        let mut matched = false;
        match cli.all {
            true => {
                if line.contains("<agmd:") {
                    matched = true;
                }
            }
            false => {
                if line.contains(&agmd_str) {
                    matched = true;
                }
            }
        }
        if matched {
            let done = line.contains(" [x]");
            items.push((done, line.trim().to_string()));
        }
    }

    items
}
