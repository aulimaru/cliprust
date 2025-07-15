use crate::config::Config;
use blake3::Hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct ClipboardHistory {
    history: Vec<Hash>,
    index_map: HashMap<Hash, usize>,
    bytes_map: HashMap<usize, Entry>,
    index_counter: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Preview {
    Text(String),
    Thumb(String, String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Entry {
    file: usize,
    preview: Preview,
}

impl Entry {
    fn from_bytes(bytes: &[u8], index: usize, config: &Config) -> Self {
        let path = config.db_dir_path.join(index.to_string());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, bytes).unwrap();
        Entry {
            file: index,
            preview: Preview::from_bytes(bytes, index, config),
        }
    }

    fn as_bytes(&self, config: &Config) -> Vec<u8> {
        let path = config.db_dir_path.join(self.file.to_string());
        std::fs::read(path).unwrap()
    }

    fn remove_file(&self, config: &Config) {
        let path = config.db_dir_path.join(self.file.to_string());
        let preview = &self.preview;
        preview.remove_file(config);
        std::fs::remove_file(path).unwrap();
    }
}

impl Preview {
    fn to_preview(&self, index: usize, config: &Config) -> String {
        let preview = match self {
            Preview::Text(preview) => text_with_limit(preview, config.preview_width),
            Preview::Thumb(preview, file) => {
                if !config.generate_thumb {
                    text_with_limit(preview, config.preview_width)
                } else {
                    let path = config.db_dir_path.join("thumbs").join(file);
                    let preview = format!(
                        ":img:{}:text:{}",
                        path.to_str().unwrap(),
                        text_with_limit(preview, config.preview_width)
                    );
                    preview
                }
            }
        };
        index.to_string() + "\t" + &preview
    }

    fn from_bytes(bytes: &[u8], index: usize, config: &Config) -> Self {
        let preview = bytes_to_preview(bytes, config);
        if let Some(kind) = infer::get(bytes) {
            match kind.matcher_type() {
                infer::MatcherType::Image => {
                    let file = format!("{}.{}", index, kind.extension());
                    let path = config.db_dir_path.join("thumbs").join(&file);
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).unwrap();
                    }
                    let img = image::load_from_memory(bytes).unwrap();
                    let img = img.resize(256, 256, image::imageops::FilterType::Lanczos3);
                    img.save(&path).unwrap();
                    Preview::Thumb(preview, file)
                }
                _ => Preview::Text(preview),
            }
        } else {
            Preview::Text(preview)
        }
    }

    fn remove_file(&self, config: &Config) {
        if let Preview::Thumb(_, file) = self {
            let path = config.db_dir_path.join("thumbs").join(file);
            std::fs::remove_file(path).unwrap();
        }
    }
}

impl ClipboardHistory {
    pub fn new() -> Self {
        ClipboardHistory {
            history: Vec::new(),
            bytes_map: HashMap::new(),
            index_map: HashMap::new(),
            index_counter: 1,
        }
    }

    pub fn add_entry(&mut self, content: &[u8], config: &Config) {
        if self.history.len() >= config.max_items {
            self.remove_oldest(config);
        }
        let hash = bytes_to_hash(&content);
        if let Some(_index) = self.dedupe_by_hash(hash) {
            if let Some(i) = self.dedupe(&content, config) {
                self.history.remove(i);
                self.history.push(hash);
                return;
            }
            // hash collision (This happens?)
            self.index_map.remove(&hash);
            self.history.retain(|&h| h != hash);
        }
        self.history.push(hash);
        self.index_map.insert(hash, self.index_counter);
        self.bytes_map.insert(
            self.index_counter,
            Entry::from_bytes(&content, self.index_counter, config),
        );
        self.index_counter += 1;
    }

    pub fn dedupe(&mut self, content: &[u8], config: &Config) -> Option<usize> {
        let dedupe_depth = std::cmp::min(self.history.len(), config.max_dedupe_depth);
        for i in (self.history.len() - dedupe_depth..self.history.len()).rev() {
            let index = self.index_map.get(&self.history[i]).unwrap();
            let entry = self.bytes_map.get(&index).unwrap();
            if entry.as_bytes(config) == content {
                return Some(i);
            }
        }
        None
    }

    pub fn dedupe_by_hash(&mut self, hash: Hash) -> Option<usize> {
        if let Some(&index) = self.index_map.get(&hash) {
            return Some(index);
        }
        None
    }

    fn remove_oldest(&mut self, config: &Config) {
        let index = self.index_map.get(&self.history[0]).unwrap();
        self.delete_entry(*index, config);
    }

    pub fn get_entry(&self, index: usize, config: &Config) -> Result<Vec<u8>, ()> {
        self.bytes_map
            .get(&index)
            .map(|entry| entry.as_bytes(config))
            .ok_or(())
    }

    pub fn list_entries(&self, config: &Config) {
        for hash in self.history.iter().rev() {
            let index = self.index_map.get(hash).expect("No index for hash");
            if let Some(entry) = self.bytes_map.get(index) {
                println!("{}", entry.preview.to_preview(*index, config));
            }
        }
    }

    pub fn delete_entry(&mut self, index: usize, config: &Config) {
        if let Some(i) = self
            .history
            .iter()
            .position(|x| *self.index_map.get(x).unwrap() == index)
        {
            let index = self
                .index_map
                .get(&self.history.remove(i))
                .expect("No index for hash");
            if let Some(entry) = self.bytes_map.get(index) {
                entry.remove_file(config);
            }
            self.bytes_map.remove(index);
        }
    }

    pub fn last(&self, config: &Config) -> String {
        let index = self
            .index_map
            .get(self.history.last().expect("No entries in history"))
            .expect("No index for last hash");
        let preview = &self
            .bytes_map
            .get(index)
            .expect("No entry for index")
            .preview;
        preview.to_preview(*index, config)
    }

    pub fn second_last(&self, config: &Config) -> String {
        if self.history.len() < 2 {
            panic!("Less than 2 entries in history");
        }
        let index = self
            .index_map
            .get(&self.history[self.history.len() - 2])
            .expect("No index for second last hash");
        let preview = &self
            .bytes_map
            .get(index)
            .expect("No entry for index")
            .preview;
        preview.to_preview(*index, config)
    }

    pub fn clear(&mut self, config: &Config) {
        let hashes: Vec<Hash> = std::mem::take(&mut self.history);

        for index in hashes.iter().filter_map(|h| self.index_map.remove(h)) {
            if let Some(entry) = self.bytes_map.remove(&index) {
                entry.remove_file(config);
            }
        }
    }

    pub fn to_file(&self, path: &Path) {
        let path = path.join("db");
        let encoded: Vec<u8> = bincode::serialize(&self).unwrap();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, encoded).unwrap();
    }

    pub fn from_file(path: &Path) -> Self {
        let path = path.join("db");
        if !path.exists() {
            return ClipboardHistory::new();
        }
        let encoded = std::fs::read(path).unwrap();
        bincode::deserialize(&encoded).unwrap()
    }
}

fn bytes_to_preview(bytes: &[u8], config: &Config) -> String {
    if let Some(kind) = infer::get(bytes) {
        format!("{} {}", kind.mime_type(), size_to_string(bytes.len()))
    } else {
        let preview: String = String::from_utf8_lossy(bytes).chars().take(500).collect();
        preview.trim().replace('\n', "↵ ")
    }
}

fn size_to_string(size: usize) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KiB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", size as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.1} GiB", size as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

fn text_with_limit(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.into();
    }
    text.chars().take(limit).collect::<String>() + "..."
}

fn bytes_to_hash(bytes: &[u8]) -> blake3::Hash {
    blake3::hash(bytes)
}
