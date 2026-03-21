#[derive(Serialize, Deserialize, Debug)]
pub struct FileItem {
    pub modified: Option<SystemTime>,
    pub hash: [u8; 32],
    pub items: Vec<String>,
}

impl FileItem {
    pub fn new(modified: Option<SystemTime>, text: &str) -> Self {
        let hash = Sha256::digest(text).into();
        Self {
            modified,
            hash,
            items: vec![],
        }
    }
}

pub struct FileDatabase {
    #[allow(unused)]
    db: Database,
    tree: Keyspace,
}

impl FileDatabase {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let db = fjall::Database::builder(path).open().unwrap();
        let tree = db
            .keyspace("items", fjall::KeyspaceCreateOptions::default)
            .unwrap();
        Self { db, tree }
    }

    pub fn get(&self, path: &Path) -> fjall::Result<Option<FileItem>> {
        let Some(item) = self.tree.get(path.as_os_str().as_bytes())? else {
            return Ok(None);
        };
        let fi = serde_json::from_slice::<FileItem>(&item).expect("should deserialize");
        Ok(Some(fi))
    }

    pub fn insert(&self, path: &Path, item: &FileItem) -> fjall::Result<()> {
        let serialized = serde_json::to_vec(item).expect("fail to serialize");
        self.tree.insert(path.as_os_str().as_bytes(), serialized)
    }
}
