use anyhow::Result;
use lmdb::{Database, Environment, EnvironmentBuilder, EnvironmentFlags};
use std::{fs, path::Path, sync::Arc};

#[derive(Clone)]
pub struct EnvDb {
    pub env: Arc<Environment>,
    pub db: Arc<Database>,
}

#[derive(Clone)]
pub struct Databases {
    pub convos: EnvDb,
    pub messages: EnvDb,
}

impl EnvDb {
    pub fn open(path: &str, size: usize) -> Result<Self> {
        let p = Path::new(path);
        if !p.exists() {
            fs::create_dir_all(p)?;
        }
        let mut builder: EnvironmentBuilder = Environment::new();
        builder.set_map_size(size);
        builder.set_flags(EnvironmentFlags::NO_META_SYNC | EnvironmentFlags::NO_TLS);
        builder.set_max_readers(1024);
        let env = Arc::new(EnvironmentBuilder::open(&builder, p)?);
        let db = Arc::new(env.open_db(None)?);
        Ok(Self { env, db })
    }
}

impl Databases {
    pub fn init() -> Result<Self> {
        Ok(Self {
            convos: EnvDb::open("lmdb_convos", 1 << 32)?, // 4 GiB
            messages: EnvDb::open("lmdb_messages", 1 << 33)?, // 8 GiB
        })
    }
}
