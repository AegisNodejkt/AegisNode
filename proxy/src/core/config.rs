use config::{Config, File};
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize, Clone)]
pub struct PrivacyRule {
    pub name: String,
    #[serde(rename = "type")]
    pub rule_type: String, // "pattern" or "entity"
    pub value: String,
    pub action: Option<String>,
    #[serde(default)]
    pub replace: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DestinationProvider {
    pub name: String,
    pub endpoint: String,
    pub api_key: String,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub destination: DestinationProvider,
    pub rules: Vec<PrivacyRule>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            destination: DestinationProvider {
                name: "openai".to_string(),
                endpoint: "https://api.openai.com/v1".to_string(),
                api_key: "".to_string(),
                model: Some("gpt-3.5-turbo".to_string()),
            },
            rules: vec![],
        }
    }
}

pub type SharedConfig = Arc<RwLock<AppConfig>>;

pub fn load_config(path: &str) -> AppConfig {
    let builder = Config::builder()
        .add_source(config::Environment::with_prefix("SHIELD"))
        .add_source(File::with_name(path).required(false));

    match builder.build() {
        Ok(cfg) => cfg.try_deserialize::<AppConfig>().unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub async fn watch_config(path: String, config_store: SharedConfig) {
    let (tx, mut rx) = mpsc::channel(1);
    
    // Create a watcher that sends events to the channel
    let mut watcher = RecommendedWatcher::new(move |res| {
        if let Ok(event) = res {
            let _ = tx.blocking_send(event);
        }
    }, notify::Config::default()).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    if let Err(e) = watcher.watch(Path::new(&path), RecursiveMode::NonRecursive) {
        eprintln!("Error watching file: {:?}", e);
        return;
    }

    println!("Watching config file: {}", path);

    // Keep the watcher alive by moving it into the loop or holding it
    // But since we are in an async task, we need to process the channel
    
    tokio::spawn(async move {
        // We need to keep `watcher` alive. 
        // In this closure, we move `watcher`.
        // Wait, `watcher` needs to stay in scope.
        let _watcher = watcher; 
        
        while let Some(event) = rx.recv().await {
            // event is Event, check if it's a modification
            if event.kind.is_modify() {
                println!("Config file changed, reloading...");
                let new_config = load_config(&path);
                {
                    let mut w = config_store.write().unwrap();
                    *w = new_config;
                }
                println!("Config reloaded successfully.");
            }
        }
    });
}


pub fn read_config(config_store: &SharedConfig) -> AppConfig {
    config_store.read().unwrap().clone()
}