use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::RwLock;

use minijinja::{Environment, Error, State};
use sha2::{Digest, Sha256};

#[derive(Debug, Default)]
pub struct AssetLoader {
    cache: RwLock<HashMap<String, String>>,
}

impl AssetLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn asset_path(&self, path: &str) -> String {
        let mut cache = self.cache.write().unwrap();
        if let Some(hashed_path) = cache.get(path) {
            return hashed_path.clone();
        }

        let file_path = Path::new("static").join(path);
        if let Ok(contents) = fs::read(file_path) {
            let mut hasher = Sha256::new();
            hasher.update(contents);
            let hash = hasher.finalize();
            let hashed_path = format!("/static/{}?v={:x}", path, hash);
            cache.insert(path.to_string(), hashed_path.clone());
            hashed_path
        } else {
            format!("/static/{}", path)
        }
    }

    pub fn register<'a>(&self, env: &mut Environment<'a>) {
        let loader = self.clone();
        env.add_function("asset", move |_state: &State, path: String| -> Result<String, Error> {
            Ok(loader.asset_path(&path))
        });
    }
}

impl Clone for AssetLoader {
    fn clone(&self) -> Self {
        AssetLoader {
            cache: RwLock::new(self.cache.read().unwrap().clone()),
        }
    }
}