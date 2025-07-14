use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minecraft models for handling version manifests, version details, and asset indices.
/// These structs are used to deserialize JSON data from the Minecraft API.
#[derive(Deserialize, Serialize)]
pub struct VersionManifest {
    pub versions: Vec<VersionInfo>,
}

#[derive(Deserialize, Serialize)]
pub struct VersionInfo {
    pub id: String,
    pub url: String,
}

#[derive(Deserialize, Serialize)]
pub struct VersionDetails {
    #[allow(dead_code)]
    pub downloads: Downloads,
    #[allow(dead_code)]
    pub libraries: Vec<Library>,
    #[serde(rename = "mainClass")]
    #[allow(dead_code)]
    pub main_class: String,
    #[serde(rename = "assetIndex")]
    #[allow(dead_code)]
    pub asset_index: AssetIndex,
}

#[derive(Deserialize, Serialize)]
pub struct Downloads {
    #[allow(dead_code)]
    pub client: DownloadInfo,
}

#[derive(Deserialize, Serialize)]
pub struct DownloadInfo {
    #[allow(dead_code)]
    pub url: String,
}

#[derive(Deserialize, Serialize)]
pub struct AssetIndex {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub url: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Library {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub downloads: Option<LibraryDownloads>,
    #[allow(dead_code)]
    pub natives: Option<HashMap<String, String>>,
    #[allow(dead_code)]
    pub extract: Option<ExtractRule>,
    #[allow(dead_code)]
    pub rules: Option<Vec<Rule>>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ExtractRule {
    #[allow(dead_code)]
    pub exclude: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Rule {
    #[allow(dead_code)]
    pub action: String,
    #[allow(dead_code)]
    pub os: Option<OsRule>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LibraryDownloads {
    #[allow(dead_code)]
    pub artifact: Option<Artifact>,
    #[allow(dead_code)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Deserialize, Serialize)]
pub struct AssetIndexManifest {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize, Serialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct OsRule {
    #[allow(dead_code)]
    pub name: Option<String>,
}
