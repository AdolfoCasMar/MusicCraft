//! Persistent playlists. Local file handles are session-only and are not serialized.
use serde_json::Value;
use std::{collections::BTreeMap, io, path::PathBuf};

pub(crate) struct PlaylistStore {
    path: PathBuf,
    items: BTreeMap<String, Vec<Value>>,
    readable: bool,
}
impl PlaylistStore {
    pub(crate) fn open(path: PathBuf) -> Self {
        match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(items) => Self {
                    path,
                    items,
                    readable: true,
                },
                Err(_) => Self {
                    path,
                    items: BTreeMap::new(),
                    readable: false,
                },
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Self {
                path,
                items: BTreeMap::new(),
                readable: true,
            },
            Err(_) => Self {
                path,
                items: BTreeMap::new(),
                readable: false,
            },
        }
    }
    pub(crate) fn names(&self) -> Vec<String> {
        self.items.keys().cloned().collect()
    }
    pub(crate) fn get(&self, name: &str) -> Option<&Vec<Value>> {
        self.items.get(name)
    }
    fn commit(&mut self, next: BTreeMap<String, Vec<Value>>) -> io::Result<()> {
        if !self.readable {
            return Err(io::Error::other(
                "No se pueden leer las playlists guardadas",
            ));
        }
        let bytes = serde_json::to_vec_pretty(&next).map_err(io::Error::other)?;
        std::fs::write(&self.path, bytes)?;
        self.items = next;
        Ok(())
    }
    pub(crate) fn save(&mut self, name: &str, queue: &[Value]) -> io::Result<usize> {
        let name = name.trim();
        if name.is_empty() || name.len() > 80 {
            return Err(io::Error::other("Nombre de playlist no válido"));
        }
        let entries: Vec<Value> = queue
            .iter()
            .filter(|x| x["kind"] != "local")
            .cloned()
            .collect();
        let excluded = queue.len() - entries.len();
        let mut next = self.items.clone();
        next.insert(name.to_owned(), entries);
        self.commit(next)?;
        Ok(excluded)
    }
    pub(crate) fn delete(&mut self, name: &str) -> io::Result<()> {
        let mut next = self.items.clone();
        next.remove(name);
        self.commit(next)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture(name: &str) -> PathBuf {
        let base = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target"));
        std::fs::create_dir_all(&base).unwrap();
        base.join(format!("playlist-test-{}-{name}.json", std::process::id()))
    }
    #[test]
    fn persists_youtube_without_serializing_local_file_handles() {
        let path = fixture("roundtrip");
        let mut store = PlaylistStore::open(path.clone());
        let excluded = store
            .save(
                "Ejemplo",
                &[
                    json!({"id":"a","kind":"songs"}),
                    json!({"id":0,"kind":"local"}),
                ],
            )
            .unwrap();
        assert_eq!(excluded, 1);
        let reopened = PlaylistStore::open(path.clone());
        assert_eq!(reopened.get("Ejemplo").unwrap().len(), 1);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn corrupt_file_is_not_overwritten() {
        let path = fixture("invalid");
        std::fs::write(&path, b"{bad data").unwrap();
        let mut store = PlaylistStore::open(path.clone());
        assert!(store.save("Prueba", &[]).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{bad data");
        std::fs::remove_file(path).unwrap();
    }
}
