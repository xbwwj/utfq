use std::path::absolute;

use chrono::Local;
use clap::Parser;
use osc8::Hyperlink;
use url::Url;

use crate::{cli::Cli, files::load_markdown_files};

fn main() {
    let cli = Cli::parse();

    let entries = load_markdown_files();

    if cli.all {
        for (path, vtodos) in entries {
            if vtodos.len() > 0 {
                println!("==== {} ====", path.display());
                for vtodo in vtodos {
                    println!("{}", vtodo);
                }
            }
        }
    } else {
        let today = Local::now().date_naive();

        // today undone only
        for (path, vtodos) in entries {
            let mut filtered = vec![];

            for vtodo in vtodos {
                if vtodo.checked {
                    continue;
                }
                let Some(agmd) = &vtodo.agmd else {
                    continue;
                };
                let contains_today = match (agmd.start, agmd.due) {
                    (None, None) => false,
                    (None, Some(due)) => due >= today,
                    (Some(start), None) => start <= today,
                    (Some(start), Some(due)) => (start..=due).contains(&today),
                };
                if contains_today {
                    filtered.push(vtodo);
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
}

mod cli {
    use clap::Parser;

    #[derive(Debug, Parser)]
    pub struct Cli {
        #[arg(short, long)]
        pub all: bool,
        #[arg(short('d'), long("done"))]
        pub show_done: bool,
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
                    (Some(start), Some(due)) => write!(f, "start={start};due={due}")?,
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
        if let Some(capture) = re.captures_iter(input).next() {
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
            let capture = re.captures_iter(component).next()?;
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
