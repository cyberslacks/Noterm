use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub relative_path: String,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub expanded: bool,
}

impl FileNode {
    pub fn display_name(&self) -> &str {
        &self.name
    }
}

pub fn scan_dir(notes_dir: &Path, show_hidden: bool) -> Vec<FileNode> {
    let mut nodes = Vec::new();
    collect_entries(notes_dir, notes_dir, 0, show_hidden, &mut nodes);
    nodes
}

fn collect_entries(
    root: &Path,
    dir: &Path,
    depth: usize,
    show_hidden: bool,
    nodes: &mut Vec<FileNode>,
) {
    let Ok(mut entries) = std::fs::read_dir(dir) else {
        return;
    };

    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();

    while let Some(Ok(entry)) = entries.next() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            dirs.push(path);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            files.push(path);
        }
    }

    dirs.sort();
    files.sort();

    for dir_path in dirs {
        let name = dir_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let relative = dir_path.strip_prefix(root).unwrap_or(&dir_path).to_string_lossy().to_string();
        nodes.push(FileNode {
            path: dir_path.clone(),
            relative_path: relative,
            name,
            is_dir: true,
            depth,
            expanded: true,
        });
        collect_entries(root, &dir_path, depth + 1, show_hidden, nodes);
    }

    for file_path in files {
        let name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let relative = file_path.strip_prefix(root).unwrap_or(&file_path).to_string_lossy().to_string();
        nodes.push(FileNode {
            path: file_path,
            relative_path: relative,
            name,
            is_dir: false,
            depth,
            expanded: false,
        });
    }
}
