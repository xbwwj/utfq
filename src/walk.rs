use ignore::{DirEntry, Error, Walk, WalkBuilder, types::TypesBuilder};

pub fn build_walk() -> Walk {
    let types = TypesBuilder::new()
        .add_defaults()
        .select("markdown")
        .build()
        .unwrap();

    WalkBuilder::new(".").types(types).build()
}

pub fn entry_is_file(entry: &DirEntry) -> bool {
    match entry.file_type() {
        None => false,
        Some(ft) => ft.is_file(),
    }
}

pub fn build_walk_filtered() -> impl Iterator<Item = Result<DirEntry, Error>> {
    build_walk().filter(|r| match r {
        Ok(entry) => entry_is_file(entry),
        Err(_) => true,
    })
}
