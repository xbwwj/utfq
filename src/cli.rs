use chrono::{Duration, Local, NaiveDate};
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cli {
    /// Which date to show
    #[arg(
        allow_hyphen_values = true,
        value_parser = parse_date_arg,
        default_value_t = Local::now().date_naive()
    )]
    pub date: NaiveDate,
    /// List all agmd items
    #[arg(short, long, default_value_t = false)]
    pub all: bool,
    /// List done items
    #[arg(short, long, default_value_t = false)]
    pub done: bool,
}

fn parse_date_arg(s: &str) -> Result<NaiveDate, String> {
    match s.parse::<i64>() {
        Ok(relative) => {
            let today = Local::now().date_naive();
            today
                .checked_add_signed(Duration::days(relative))
                .ok_or_else(|| "date out of range".to_string())
        }
        Err(_) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| "expect YYYY-MM-DD or relative".to_string()),
    }
}
