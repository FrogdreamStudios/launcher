use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionDetails {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    #[serde(rename = "minecraftArguments", skip_serializing_if = "Option::is_none")]
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub libraries: Vec<Library>,
    pub downloads: Downloads,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndex,
    pub assets: String,
    #[serde(rename = "javaVersion", skip_serializing_if = "Option::is_none")]
    pub java_version: Option<JavaVersion>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Arguments {
    pub game: Vec<ArgumentValue>,
    pub jvm: Vec<ArgumentValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    String(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValueInner,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgumentValueInner {
    String(String),
    Array(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Library {
    pub name: String,
    pub downloads: LibraryDownloads,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<HashMap<String, String>>,
    pub extract: Option<ExtractRules>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Artifact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractRules {
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Downloads {
    pub client: Option<Artifact>,
    pub server: Option<Artifact>,
    #[serde(rename = "client_mappings")]
    pub client_mappings: Option<Artifact>,
    #[serde(rename = "server_mappings")]
    pub server_mappings: Option<Artifact>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetManifest {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

impl VersionManifest {
    pub const MANIFEST_URL: &'static str =
        "https://launchermeta.mojang.com/mc/game/version_manifest.json";

    pub fn get_version(&self, version_id: &str) -> Option<&VersionInfo> {
        self.versions.iter().find(|v| v.id == version_id)
    }
}

impl Rule {
    pub fn matches(&self, os_name: &str, os_arch: &str, features: &HashMap<String, bool>) -> bool {
        let mut matches = true;

        // Check OS rules
        if let Some(os) = &self.os {
            if let Some(name) = &os.name {
                matches &= name == os_name;
            }
            if let Some(arch) = &os.arch {
                matches &= arch == os_arch;
            }
        }

        // Check feature rules
        if let Some(rule_features) = &self.features {
            for (feature, required) in rule_features {
                let has_feature = features.get(feature).unwrap_or(&false);
                matches &= has_feature == required;
            }
        }

        match self.action.as_str() {
            "allow" => matches,
            "disallow" => !matches,
            _ => false,
        }
    }
}

impl Library {
    pub fn should_use(
        &self,
        os_name: &str,
        os_arch: &str,
        features: &HashMap<String, bool>,
    ) -> bool {
        if let Some(rules) = &self.rules {
            // Default is disallowed if there are rules
            let mut allowed = false;

            // Process rules in order
            for rule in rules {
                if rule.matches(os_name, os_arch, features) {
                    match rule.action.as_str() {
                        "allow" => allowed = true,
                        "disallow" => allowed = false,
                        _ => {}
                    }
                }
            }

            allowed
        } else {
            true
        }
    }
}
