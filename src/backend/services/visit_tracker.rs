use crate::backend::utils::paths::get_launcher_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteVisit {
    pub name: String,
    pub url: String,
    pub icon_key: String,
    pub last_visited: u64, // Unix timestamp
    pub visit_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisitData {
    pub sites: HashMap<String, SiteVisit>,
}

pub struct VisitTracker {
    data: Arc<Mutex<VisitData>>,
    config_path: PathBuf,
}

impl VisitTracker {
    pub fn new() -> Self {
        let config_path = get_launcher_dir()
            .unwrap_or_else(|_| PathBuf::from("DreamLauncher"))
            .join("visit_history.json");
        let data = Self::load_data(&config_path);

        Self {
            data: Arc::new(Mutex::new(data)),
            config_path,
        }
    }

    fn load_data(path: &PathBuf) -> VisitData {
        if path.exists()
            && let Ok(content) = fs::read_to_string(path)
            && let Ok(data) = serde_json::from_str(&content)
        {
            return data;
        }

        // Return default data with initial sites
        let mut data = VisitData::default();

        data.sites.insert(
            "minecraft".to_string(),
            SiteVisit {
                name: "Minecraft".to_string(),
                url: "https://www.minecraft.net".to_string(),
                icon_key: "minecraft_icon".to_string(),
                last_visited: 0, // Never visited
                visit_count: 0,
            },
        );

        data.sites.insert(
            "minecraft_wiki".to_string(),
            SiteVisit {
                name: "Minecraft Wiki".to_string(),
                url: "https://minecraft.wiki/".to_string(),
                icon_key: "minecraft_wiki_icon".to_string(),
                last_visited: 0, // Never visited
                visit_count: 0,
            },
        );

        data.sites.insert(
            "planet_minecraft".to_string(),
            SiteVisit {
                name: "Planet Minecraft".to_string(),
                url: "https://www.planetminecraft.com".to_string(),
                icon_key: "planet_minecraft_icon".to_string(),
                last_visited: 0, // Never visited
                visit_count: 0,
            },
        );

        data.sites.insert(
            "curseforge".to_string(),
            SiteVisit {
                name: "CurseForge".to_string(),
                url: "https://www.curseforge.com/minecraft".to_string(),
                icon_key: "curseforge_icon".to_string(),
                last_visited: 0, // Never visited
                visit_count: 0,
            },
        );

        data.sites.insert(
            "namemc".to_string(),
            SiteVisit {
                name: "NameMC".to_string(),
                url: "https://namemc.com".to_string(),
                icon_key: "namemc_icon".to_string(),
                last_visited: 0, // Never visited
                visit_count: 0,
            },
        );

        data
    }

    fn save_data(&self) {
        if let Ok(data) = self.data.lock()
            && let Ok(json) = serde_json::to_string_pretty(&*data)
        {
            // Create a config directory if it doesn't exist
            if let Some(parent) = Path::new(&self.config_path).parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&self.config_path, json);
        }
    }

    pub fn record_visit(&self, site_key: &str) {
        if let Ok(mut data) = self.data.lock()
            && let Some(site) = data.sites.get_mut(site_key)
        {
            site.last_visited = Self::current_timestamp();
            site.visit_count += 1;
            drop(data);
            self.save_data();
        }
    }

    pub fn get_sorted_sites(&self) -> Vec<SiteVisit> {
        if let Ok(data) = self.data.lock() {
            let mut sites: Vec<SiteVisit> = data.sites.values().cloned().collect();
            // Sort by visit_count first (visited vs. unvisited), then by last_visited descending
            sites.sort_by(|a, b| {
                match (a.visit_count > 0, b.visit_count > 0) {
                    (true, false) => std::cmp::Ordering::Less, // Visited sites come first
                    (false, true) => std::cmp::Ordering::Greater, // Unvisited sites come last
                    (true, true) => b.last_visited.cmp(&a.last_visited), // Both visited: sort by most recent
                    (false, false) => a.name.cmp(&b.name), // Both unvisited: sort alphabetically
                }
            });
            sites
        } else {
            Vec::new()
        }
    }

    pub fn format_time_ago(timestamp: u64) -> String {
        let now = Self::current_timestamp();
        let diff = now.saturating_sub(timestamp);

        match diff {
            0..=59 => "Visited just now".to_string(),
            60..=119 => "Visited 1 minute ago".to_string(),
            120..=3599 => format!("Visited {} minutes ago", diff / 60),
            3600..=7199 => "Visited 1 hour ago".to_string(),
            7200..=86399 => format!("Visited {} hours ago", diff / 3600),
            86400..=172799 => "Visited 1 day ago".to_string(),
            172800..=2591999 => format!("Visited {} days ago", diff / 86400),
            2592000..=5183999 => "Visited 1 month ago".to_string(),
            _ => format!("Visited {} months ago", diff / 2592000),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

impl Default for VisitTracker {
    fn default() -> Self {
        Self::new()
    }
}
