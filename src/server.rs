use crate::config::Config;
use colored::*;
use daemonize::Daemonize;
use reqwest::blocking::Client;
use serde_json::Value;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::iterator::Signals;
use std::fs::File;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::{
    self, fs,
    path::PathBuf,
    sync::mpsc::{self},
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

        let home_dir = dirs::home_dir().ok_or("Unable to determine home directory")?;
        let stdout_path = home_dir.join("jp_tools_daemon.out");
        let stderr_path = home_dir.join("jp_tools_daemon.err");
        let stdout = File::create(&stdout_path)?;
        let stderr = File::create(&stderr_path)?;
        let daemonize = Daemonize::new()
            .pid_file(&self.pid_path)
            .stdout(stdout)
            .stderr(stderr);

        let url = std::env::var("TOOL_DASHBOARD")
            .expect("You should set the TOOL_DASHBOARD env variable");
        println!("Starting server...");
        println!("  Output Log: {}", stdout_path.display());
        println!("  Error Log: {}", stderr_path.display());

        match daemonize.start() {
            Ok(_) => {
                // Daemon started successfully, PID file should now exist
                if let Ok(pid) = fs::read_to_string(&self.pid_path) {
                    println!(
                        "{} Server started successfully (PID: {})",
                        "✓".green(),
                        pid.trim()
                    );
                }

                let (tx, rx) = mpsc::channel();
                let mut signals = Signals::new(TERM_SIGNALS)
                    .map_err(|e| format!("Failed to setup signal handling: {}", e))?;

                thread::spawn(move || {
                    for _sig in signals.forever() {
                        let _ = tx.send(()); // notify main thread
                    }
                });

                // keep running loop in main thread, call cleanup when signal received
                loop {
                    if rx.try_recv().is_ok() {
                        println!("STOPPING SERVER");
                        self.cleanup();
                        std::process::exit(0);
                    }

                    // Create client for each request to avoid runtime conflicts
                    match self.post_updates(&url) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error posting updates: {}", e);
                        }
                    }
                    thread::sleep(Duration::from_secs(5));
                }
            }
            Err(e) => eprintln!("{} Failed to start daemon: {}", "✗".red(), e),
        }
        println!("Server Stopped");
        self.cleanup();
        Ok(())
    }

    fn post_updates(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.config.reload()?;
        if self.config.toml_data.len() == 0 {
            return Err("No Files to watch".into());
        }

        let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

        let mut all_data: Vec<Value> = vec![];
        for (key, val) in &self.config.toml_data {
            let content = fs::read_to_string(&val.status_file)
                .map_err(|e| format!("Failed to read status file {}: {}", val.status_file, e))?;
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
            if Self::process_exists(&pid) {
                return true;
            } else {
                if self.pid_path.exists() {
                    let _ = fs::remove_file(&self.pid_path);
                }
                return false;
            }
        }
        false
    }

    fn process_exists(pid: &str) -> bool {
        Command::new("kill")
            .args(["-0", pid])
            .output()
            .map_or(false, |output| output.status.success())
    }

    fn cleanup(&self) {
        if self.pid_path.exists() {
            fs::remove_file(&self.pid_path).expect("Unable to delete pid file");
        }
    }
}
