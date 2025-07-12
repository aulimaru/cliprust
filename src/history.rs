use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct ClipboardHistory {
    history: Vec<usize>,
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
        let mut path = config.db_dir_path.clone();
        let file = index;
        path.push(file.to_string());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, bytes).unwrap();
        Entry {
            file,
            preview: Preview::from_bytes(bytes, index, config),
        }
    }

    fn as_bytes(&self, config: &Config) -> Vec<u8> {
        let mut path = config.db_dir_path.clone();
        path.push(self.file.to_string());
        std::fs::read(path).unwrap()
    }

    fn remove_file(&self, config: &Config) {
        let preview = &self.preview;
        preview.remove_file(config);
        let mut path = config.db_dir_path.clone();
        path.push(self.file.to_string());
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
                    let mut path = config.db_dir_path.clone();
                    path.push("thumbs");
                    path.push(file);
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
                    let mut path = config.db_dir_path.clone();
                    let file = format!("{}.{}", index, kind.extension());
                    path.push("thumbs");
                    path.push(&file);
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
            let mut path = config.db_dir_path.clone();
            path.push("thumbs");
            path.push(file);
            std::fs::remove_file(path).unwrap();
        }
    }
}

impl ClipboardHistory {
    pub fn new() -> Self {
        ClipboardHistory {
            history: Vec::new(),
            bytes_map: HashMap::new(),
            index_counter: 1,
        }
    }

    pub fn add_entry(&mut self, content: Vec<u8>, config: &Config) {
        while self.history.len() >= config.max_items {
            self.remove_oldest(config);
        }
        let dedupe_depth = std::cmp::min(self.history.len(), config.max_dedupe_depth);
        for i in (self.history.len() - dedupe_depth..self.history.len()).rev() {
            let index = self.history[i];
            let entry = self.bytes_map.get(&index).unwrap();
            if entry.as_bytes(config) == content {
                self.history.remove(i);
                self.history.push(index);
                return;
            }
        }
        self.history.push(self.index_counter);
        self.bytes_map.insert(
            self.index_counter,
            Entry::from_bytes(&content, self.index_counter, config),
        );
        self.index_counter += 1;
    }

    fn remove_oldest(&mut self, config: &Config) {
        let index = self.history[0];
        self.delete_entry(index, config);
    }

    pub fn get_entry(&self, index: usize, config: &Config) -> Result<Vec<u8>, ()> {
        self.bytes_map
            .get(&index)
            .map(|entry| entry.as_bytes(config))
            .ok_or(())
    }

    pub fn list_entries(&self, config: &Config) {
        for index in self.history.iter().rev() {
            if let Some(entry) = self.bytes_map.get(index) {
                println!("{}", entry.preview.to_preview(*index, config));
            }
        }
    }

    pub fn delete_entry(&mut self, index: usize, config: &Config) {
        if let Some(i) = self.history.iter().position(|&x| x == index) {
            let index = self.history.remove(i);
            if let Some(entry) = self.bytes_map.get(&index) {
                entry.remove_file(config);
            }
            self.bytes_map.remove(&index);
        }
    }

    pub fn last(&self, config: &Config) -> String {
        let index = self.history.last().expect("No entries in history");
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
        let index = &self.history[self.history.len() - 2];
        let preview = &self
            .bytes_map
            .get(index)
            .expect("No entry for index")
            .preview;
        preview.to_preview(*index, config)
    }

    pub fn clear(&mut self, config: &Config) {
        while !self.history.is_empty() {
            let index = self.history[0];
            self.delete_entry(index, config);
        }
    }

    pub fn to_file(&self, path: &PathBuf) {
        let mut path = path.clone();
        path.push("db");
        let encoded: Vec<u8> = bincode::serialize(&self).unwrap();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, encoded).unwrap();
    }

    pub fn from_file(path: &PathBuf) -> Self {
        let mut path = path.clone();
        path.push("db");
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
