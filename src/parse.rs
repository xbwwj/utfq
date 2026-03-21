use chrono::NaiveDate;

use crate::cli::Cli;

/// 解析文件，同时生成链接。
pub fn parse_file(cli: &Cli, date: NaiveDate, text: &str) -> Vec<String> {
    let mut items = vec![];
    let agmd_str = format!("<agmd:{}>", date);

    if !text.contains("<agmd:") {
        return items;
    }

    for line in text.lines() {
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
            // filter out done
            if !cli.done && line.contains(" [x] ") {
                continue;
            }

            items.push(line.trim().to_string());
        }
    }

    items
}
