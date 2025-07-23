use colored::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoConfig {
    pub status_file: String,
}

pub struct Config {
    pub file_path: String,
    pub toml_data: HashMap<String, RepoConfig>,
}

impl Config {
    pub fn create_or_load(toml_path: PathBuf) -> Self {
        if toml_path.exists() {
            return Self {
                file_path: toml_path.display().to_string(),
                toml_data: Self::read_toml(toml_path),
            };
        }
        Self {
            file_path: toml_path.display().to_string(),
            toml_data: HashMap::new(),
        }
    }

    fn read_toml(toml_path: PathBuf) -> HashMap<String, RepoConfig> {
        let content = fs::read_to_string(toml_path).expect("Unable to read toml file");
        toml::from_str(&content).unwrap()
    }

    pub fn watch_file(&mut self, path: PathBuf, repo_name: Option<String>) {
        if path.exists() {
            let final_repo_name = repo_name.unwrap_or_else(|| Self::get_repo_name(&path));
            self.toml_data.insert(
                final_repo_name.clone(),
                RepoConfig {
                    status_file: if path.extension().is_some() {
                        path.display().to_string()
                    } else {
                        path.join(Self::discover_status_file(&path).expect("No Status file found"))
                            .display()
                            .to_string()
                    },
                },
            );
            self.write_back();
            println!("Successfully added - {}", final_repo_name.green());
            return;
        }
        panic!(
            "Given file path does not exist. Please check again - {}",
            path.display()
        );
    }

    fn get_repo_name(file_path: &PathBuf) -> String {
        if file_path.extension().and_then(|os_pth| os_pth.to_str()) == Some("json") {
            return file_path
                .parent()
                .and_then(|parent_os_str| parent_os_str.file_name().unwrap().to_str())
                .unwrap()
                .to_string();
        }
        return file_path
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .unwrap()
            .to_string();
    }

    fn discover_status_file(folder_path: &PathBuf) -> Option<String> {
        fs::read_dir(folder_path)
            .ok()? // gracefully handle invalid folder
            .filter_map(Result::ok) // ignore entries that failed to read
            .map(|entry| entry.path())
            .find(|file_path| {
                file_path.extension().and_then(|ext| ext.to_str()) == Some("json")
                    && file_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.contains("status_"))
                        .unwrap_or(false)
            })
            .and_then(|p| p.file_name()?.to_str().map(String::from))
    }

    fn write_back(&self) {
        let toml_data = toml::to_string(&self.toml_data).expect("Unable to convert to TOML format");
        fs::write(&self.file_path, toml_data).expect("Unable to write to file");
    }

    pub fn list_all(&self) {
        for (key, val) in &self.toml_data {
            println!(
                "Repo - {} {} {}",
                key.green(),
                "->".bright_blue(),
                val.status_file.magenta()
            );
        }
    }

    pub fn remove(&mut self, repo_name: String) {
        if self.toml_data.contains_key(&repo_name) {
            let popped_config = self
                .toml_data
                .remove(&repo_name)
                .expect(&format!("Failed to remove - {}", repo_name));
            self.write_back();
            println!(
                "{} {} -> {}",
                "Removed".red(),
                repo_name.red().bold(),
                popped_config.status_file
            );
            return;
        }
        println!("{} does not exist", repo_name);
    }

    pub fn reload(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let toml_path = PathBuf::from(&self.file_path);
        if toml_path.exists() {
            self.toml_data = Self::read_toml(toml_path);
        }
        Ok(())
    }
}
