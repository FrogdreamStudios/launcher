use serde::Deserialize;

#[derive(Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<VersionInfo>,
}

#[derive(Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct VersionJson {
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersionField>,
}

#[derive(Deserialize)]
pub struct JavaVersionField {
    #[serde(rename = "majorVersion")]
    pub major_version: u8,
}