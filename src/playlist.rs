use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma", "ape", "aiff"
];

#[derive(Debug, Clone)]
pub struct Playlist {
    tracks: Vec<String>,
    current_index: Option<usize>,
}



impl Playlist {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            current_index: None,
        }
    }

    pub fn add_track(&mut self, path: String) -> Result<()> {
        let path_obj = Path::new(&path);
        
        if path_obj.is_file() {
            if is_audio_file(&path) {
                self.tracks.push(path);
            }
        } else if path_obj.is_dir() {
            // Recursively add all audio files from directory
            for entry in WalkDir::new(path_obj).follow_links(true) {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(path_str) = path.to_str() {
                            if is_audio_file(path_str) {
                                self.tracks.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // If this is the first track, set it as current
        if self.tracks.len() == 1 {
            self.current_index = Some(0);
        }
        
        Ok(())
    }

    pub fn add_tracks(&mut self, paths: Vec<String>) -> Result<()> {
        for path in paths {
            self.add_track(path)?;
        }
        Ok(())
    }



    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_index = None;
    }

    pub fn next(&mut self) -> Option<String> {
        if self.tracks.is_empty() {
            return None;
        }

        let next_index = match self.current_index {
            None => Some(0),
            Some(current) => {
                let next = current + 1;
                if next >= self.tracks.len() {
                    None
                } else {
                    Some(next)
                }
            }
        };

        self.current_index = next_index;
        next_index.map(|i| self.tracks[i].clone())
    }

    pub fn previous(&mut self) -> Option<String> {
        if self.tracks.is_empty() {
            return None;
        }

        let prev_index = match self.current_index {
            None => Some(0),
            Some(current) => {
                if current == 0 {
                    Some(0)
                } else {
                    Some(current - 1)
                }
            }
        };

        self.current_index = prev_index;
        prev_index.map(|i| self.tracks[i].clone())
    }

    pub fn current(&self) -> Option<String> {
        self.current_index.map(|i| self.tracks[i].clone())
    }

    pub fn get_tracks(&self) -> Vec<String> {
        self.tracks.clone()
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }






}

fn is_audio_file(path: &str) -> bool {
    let path = Path::new(path);
    
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            return AUDIO_EXTENSIONS.contains(&ext_str.to_lowercase().as_str());
        }
    }
    
    false
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}
