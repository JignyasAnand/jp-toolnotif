use crate::config::Config;
use colored::*;
use reqwest::blocking::Client;
use serde_json::Value;
use std::process::Command;
use std::time::Duration;
use std::{
    self, fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

pub struct Server {
    config: Config,
    pid_path: PathBuf,
}

impl Server {
    pub fn new(config: Config, config_dir: PathBuf) -> Self {
        Self {
            config: config,
            pid_path: config_dir.join("rs-notifier.pid"),
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_running() {
            println!("{}", "Server is already running".yellow());
            return Ok(());
        }

        let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();
        ctrlc::set_handler(move || {
            let _ = tx.send(());
        })?;
        fs::write(&self.pid_path, std::process::id().to_string())?;

        let client = Client::new();
        let url = std::env::var("TOOL_DASHBOARD")
            .expect("You should set the TOOL_DASHBOARD env variable");

        self.post_updates(&client, &url)?;
        while rx.recv_timeout(Duration::from_secs(3)).is_err() {
            self.post_updates(&client, &url)?;
        }

        println!("Server Stopped");
        self.cleanup();
        Ok(())
    }

    fn post_updates(
        &mut self,
        client: &Client,
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.config.reload()?;
        if self.config.toml_data.len() == 0 {
            return Err("No Files to watch".into());
        }
        let mut all_data: Vec<Value> = vec![];
        for (key, val) in &self.config.toml_data {
            let content = fs::read_to_string(&val.status_file).unwrap();
            let mut parsed_json: Value = serde_json::from_str(&content)?;
            parsed_json["project"] = Value::String(key.clone());
            all_data.push(parsed_json);
        }
        let response = client.post(url).json(&all_data).send()?;

        if response.status().is_success() {
            println!("Successfully sent updates");
            Ok(())
        } else {
            Err(format!("HTTP error: {}", response.status()).into())
        }
    }

    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_running() {
            println!("{}", "Server is not running".yellow());
            return Ok(());
        }
        let pid = fs::read_to_string(&self.pid_path)?;
        let u_pid: u32 = pid.trim().parse()?;
        let output = Command::new("kill").arg(u_pid.to_string()).output()?;
        if output.status.success() {
            println!("{} Server stopped (PID: {})", "✓".green(), pid);
            if self.pid_path.exists() {
                fs::remove_file(&self.pid_path)?;
            }
        } else {
            println!("{} Failed to stop server (PID: {})", "✗".red(), pid);
        };
        Ok(())
    }

    fn is_running(&self) -> bool {
        if !self.pid_path.exists() {
            return false;
        }
        if let Ok(pid) = fs::read_to_string(&self.pid_path) {
            if Self::process_exists(pid) {
                return true;
            } else {
                self.cleanup();
                return false;
            }
        }
        false
    }

    fn process_exists(pid: String) -> bool {
        Command::new("kill")
            .args(["-0", &pid])
            .output()
            .map_or(false, |output| output.status.success())
    }

    fn cleanup(&self) {
        if self.pid_path.exists() {
            fs::remove_file(&self.pid_path).expect("Unable to delete pid file");
        }
    }
}
