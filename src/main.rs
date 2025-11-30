use std::path::absolute;

use clap::Parser;
use osc8::Hyperlink;
use url::Url;

use crate::{cli::Cli, files::load_markdown_files};

fn main() {
    let cli = Cli::parse();

    let entries = load_markdown_files();

    for (path, vtodos) in entries {
        let mut filtered = vec![];

        for vtodo in vtodos {
            if !cli.done && vtodo.checked {
                continue;
            }
            match cli.malformed {
                true => {
                    if vtodo.agmd.is_none() {
                        filtered.push(vtodo);
                    }
                }
                false => {
                    let Some(agmd) = &vtodo.agmd else {
                        continue;
                    };
                    let has_intersection = cli.date_range.filter_agmd_intersection(agmd);
                    if has_intersection {
                        filtered.push(vtodo);
                    }
                }
            }
        }

        if filtered.len() > 0 {
            println!(
                "==== {}{}{} ====",
                Hyperlink::new(
                    Url::from_file_path(absolute(&path).unwrap())
                        .unwrap()
                        .as_str()
                ),
                path.display(),
                Hyperlink::END
            );
            for vtodo in filtered {
                println!("{}", vtodo);
            }
        }
    }
}

mod cli {
    use clap::Parser;

    use crate::date_range::{self, DateRangeFormat};

    #[derive(Debug, Parser)]
    pub struct Cli {
        /// Whether to show malformed.
        #[arg(short, long)]
        pub malformed: bool,
        /// Whether to show done tasks or not.
        #[arg(short, long)]
        pub done: bool,
        /// Date range filter.
        #[arg(
            allow_hyphen_values(true),
            value_parser = date_range::parse_date_range,
            default_value_t = DateRangeFormat::default(),
        )]
        pub date_range: DateRangeFormat,
    }
}

mod files {
    use std::{collections::HashMap, fs, path::PathBuf};

    use ignore::{WalkBuilder, types::TypesBuilder};

    use crate::markdown::{VTodo, parse_markdown};

    // TODO: 目前 VEvent 还没有处理
    pub fn load_markdown_files() -> HashMap<PathBuf, Vec<VTodo>> {
        let mut map = HashMap::<PathBuf, Vec<VTodo>>::new();

        // 以当前目录为根
        let root_dir = ".";

        let types = TypesBuilder::new()
            .add_defaults()
            .select("md")
            .build()
            .unwrap();

        let walker = WalkBuilder::new(root_dir)
            .add_custom_ignore_filename(".utfqignore")
            .types(types)
            .build();

        for result in walker {
            let entry = result.unwrap();
            if !entry.path().is_file() {
                continue;
            }
            let path = entry.path();
            let input = fs::read_to_string(path).unwrap();
            let vtodos = parse_markdown(&input);

            map.insert(path.to_path_buf(), vtodos);
        }

        map
    }
}

mod markdown {
    //! 本模块负责解析 markdown 中的节点。

    use std::fmt::Display;

    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    use crate::syntax::{Agmd, parse_agmd};

    /// 存储待办事项。
    #[derive(Debug)]
    pub struct VTodo {
        pub checked: bool,
        pub text: String,
        pub agmd: Option<Agmd>,
    }

    impl Display for VTodo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "- [")?;
            match self.checked {
                true => write!(f, "x")?,
                false => write!(f, " ")?,
            };
            write!(f, "] {} <agmd:", self.text)?;

            if let Some(agmd) = &self.agmd {
                match (agmd.start, agmd.due) {
                    (None, Some(due)) => write!(f, "due={due}")?,
                    (Some(start), None) => write!(f, "start={start}")?,
                    (Some(start), Some(due)) => {
                        if start == due {
                            write!(f, "{start}")?
                        } else {
                            write!(f, "start={start};due={due}")?
                        }
                    }
                    _ => {}
                }
            }

            write!(f, ">")
        }
    }

    // /// 存储事件安排。
    // #[derive(Debug)]
    // pub struct VEventPre {
    //     pub text: String,
    //     pub agmd: String,
    // }

    /// 目前所处的位置。
    ///
    /// SAX 处理的一般思路，使用 state enum 和 stack 处理。
    enum Position {
        /// 在列表项内部，是处理的主要部分。
        Item {
            /// 是否任务列表。
            checked: Option<bool>,
            /// `None` 表示没有 agmd.
            agmd: Option<String>,
            text: String,
        },
        /// 在 agmd 链接内部，这一部分的文本就不必加入了。
        /// 在栈顶端相当于一个 mask, 防止其中的文本被加入 `::Item`.
        AgmdLink,
    }

    /// 解析 markdown 文档，从中提取 amgd 任务。
    ///
    /// 目前只处理 task list, 不处理普通 list.
    pub fn parse_markdown(input: &str) -> Vec<VTodo> {
        let mut vtasks = Vec::<VTodo>::new();

        let options = Options::all();
        let parser = Parser::new_ext(input, options);

        let mut stack = Vec::<Position>::new();

        for event in parser {
            // 主要处理这些事件：
            // - `Start(Item)`: stack in new position
            // - `End(Item)`: stack out, if `is_task`, add to tasks vec
            // - `Start(Link)`: if agmd, set `agmd`, stack in `AgmdLink`
            // - `End(Link)`: if agmd, stack out `AgmdLink`
            // - `Text`: if position = item, push text
            // - `TaskListMarker`: set `is_ask`
            // 另外，使用 `Start(Del)` 来排除事项
            match event {
                Event::Start(Tag::Item) => {
                    stack.push(Position::Item {
                        checked: None,
                        agmd: None,
                        text: String::new(),
                    });
                }
                Event::End(TagEnd::Item) => {
                    let Some(Position::Item {
                        checked,
                        agmd,
                        text,
                    }) = stack.pop()
                    else {
                        panic!("expect end of link")
                    };
                    if let Some(agmd) = agmd {
                        let text = text.trim().to_string();
                        match checked {
                            Some(checked) => {
                                vtasks.push(VTodo {
                                    checked,
                                    agmd: parse_agmd(&agmd),
                                    text,
                                });
                            }
                            None => {
                                // vevents.push(VEventPre { text, agmd });
                            }
                        }
                    }
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    if let Some((_, snd)) = dest_url.split_once("agmd:")
                        && let Some(Position::Item { agmd, .. }) = stack.last_mut()
                    {
                        *agmd = Some(snd.to_string());
                        stack.push(Position::AgmdLink);
                    }
                }
                Event::End(TagEnd::Link) => {
                    stack.pop_if(|p| matches!(p, Position::AgmdLink));
                }
                Event::Text(cow_str) => {
                    let Some(Position::Item { text, .. }) = stack.last_mut() else {
                        continue;
                    };
                    text.push_str(&cow_str);
                }
                Event::TaskListMarker(is_checked) => {
                    let Some(Position::Item { checked, .. }) = stack.last_mut() else {
                        panic!("task marker should be in item");
                    };
                    *checked = Some(is_checked);
                }
                _ => {}
            }
        }

        vtasks
    }
}

mod syntax {
    //! agmd:... 后面的具体样式

    // 分析：
    //
    // - agmd:2025-11-30
    // - agmd:start=2025-11-30;due=2025-12-20
    // - agmd:due=2025-12-30
    //
    // 用什么解析格式？
    //
    // - nom: 明显是 nom 更有优势
    // - regex
    //
    // 基本可以分为三个部分：
    //
    // [$BASE];[start=$START];[due=$DUE]
    //
    //
    // NOTE: 暂时只允许完全格式

    use chrono::NaiveDate;
    use regex::Regex;

    #[derive(Debug)]
    pub struct Agmd {
        /// 开始时间。
        pub start: Option<NaiveDate>,
        /// 截至时间。
        pub due: Option<NaiveDate>,
    }

    /// ## Returns
    ///
    /// 如果是 `Option`, 表示解析出错。
    pub fn parse_agmd(input: &str) -> Option<Agmd> {
        let mut start = None;
        let mut due = None;

        // 额外处理一下 YYYY-mm-dd
        // XXX: 未来还是需要更加一致的流程
        let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").expect("fail to build regex");
        if let Some(capture) = re.captures(input) {
            let year = capture.get(1).unwrap().as_str().parse().unwrap();
            let month = capture.get(2).unwrap().as_str().parse().unwrap();
            let day = capture.get(3).unwrap().as_str().parse().unwrap();
            let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            return Some(Agmd {
                start: Some(date),
                due: Some(date),
            });
        }

        let re = Regex::new(r"(\w+)=(\d{4})-(\d{2})-(\d{2})").expect("fail to build regex");
        for component in input.split(";") {
            let capture = re.captures(component)?;
            let key = capture.get(1).unwrap().as_str();
            // XXX: too many unwrap here
            let year = capture.get(2).unwrap().as_str().parse().unwrap();
            let month = capture.get(3).unwrap().as_str().parse().unwrap();
            let day = capture.get(4).unwrap().as_str().parse().unwrap();
            let date = NaiveDate::from_ymd_opt(year, month, day)?;

            match key {
                "start" => start = Some(date),
                "due" => due = Some(date),
                _ => {}
            }
        }

        Some(Agmd { start, due })
    }
}

mod date_range {
    // Allowed formats:
    //
    // - `-1`
    // - `3`
    // - `+3`
    // - `-1..3`
    // - `-1..3`
    // - `..3`
    // - `..`
    //
    // TODO: use `.` as alias of `0`, e.g. `-1...`

    use std::fmt::Display;

    use chrono::{Local, NaiveDate, TimeDelta};
    use regex::Regex;

    use crate::syntax::Agmd;

    #[derive(Debug, Clone)]
    pub enum DateFormat {
        Relative(i64),
    }

    impl Display for DateFormat {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DateFormat::Relative(i) => i.fmt(f),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub enum DateRangeFormat {
        Single(DateFormat),
        Range(Option<DateFormat>, Option<DateFormat>),
    }

    impl Default for DateRangeFormat {
        fn default() -> Self {
            Self::Single(DateFormat::Relative(0))
        }
    }

    impl DateRangeFormat {
        pub fn filter_agmd_intersection(&self, agmd: &Agmd) -> bool {
            let today = Local::now().date_naive();
            let absolute = |d: &DateFormat| -> NaiveDate {
                let DateFormat::Relative(i) = d;
                today.checked_add_signed(TimeDelta::days(*i)).unwrap()
            };
            match self {
                DateRangeFormat::Single(d) => {
                    let d = absolute(d);
                    match (agmd.start, agmd.due) {
                        (None, None) => false,
                        (None, Some(due)) => due >= d,
                        (Some(start), None) => start <= d,
                        (Some(start), Some(due)) => (start..=due).contains(&d),
                    }
                }
                DateRangeFormat::Range(d1, d2) => {
                    let d1 = d1.as_ref().map(absolute);
                    let d2 = d2.as_ref().map(absolute);
                    match (d1, d2, agmd.start, agmd.due) {
                        // one of them is infinity
                        (None, None, _, _) | (_, _, None, None) => true,
                        (None, _, None, _) | (_, None, _, None) => true,
                        (None, Some(d2), Some(start), None)
                        | (None, Some(d2), Some(start), Some(_))
                        | (Some(_), Some(d2), Some(start), None) => d2 >= start,
                        (Some(d1), None, None, Some(due))
                        | (Some(d1), None, Some(_), Some(due))
                        | (Some(d1), Some(_), None, Some(due)) => d1 <= due,
                        (Some(d1), Some(d2), Some(start), Some(due)) => d2 >= start && d1 <= due,
                    }
                }
            }
        }
    }

    impl Display for DateRangeFormat {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DateRangeFormat::Single(date_format) => date_format.fmt(f),
                DateRangeFormat::Range(date_format, date_format1) => {
                    match (date_format, date_format1) {
                        (None, None) => write!(f, ".."),
                        (None, Some(d)) => write!(f, "..{d}"),
                        (Some(d), None) => write!(f, "{d}.."),
                        (Some(d), Some(e)) => write!(f, "{d}..{e}"),
                    }
                }
            }
        }
    }

    // XXX: use nom
    pub fn parse_date_range(
        input: &str,
    ) -> Result<DateRangeFormat, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let re1 = Regex::new(r"^[+-]?\d+$").unwrap();
        let re2 = Regex::new(r"^([+-]?\d+)?..([+-]?\d+)?$").unwrap();

        if let Some(captures) = re1.captures(input) {
            let n = captures.get(0).unwrap().as_str().parse::<i64>()?;
            return Ok(DateRangeFormat::Single(DateFormat::Relative(n)));
        }

        if let Some(captures) = re2.captures(input) {
            let n = captures
                .get(1)
                .and_then(|m| m.as_str().parse::<i64>().ok().map(DateFormat::Relative));
            let m = captures
                .get(2)
                .and_then(|m| m.as_str().parse::<i64>().ok().map(DateFormat::Relative));

            return Ok(DateRangeFormat::Range(n, m));
        }

        Err("neither single date or range".into())
    }
}
