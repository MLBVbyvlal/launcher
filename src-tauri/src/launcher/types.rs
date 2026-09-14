//! Mojang launcher-meta JSON shapes (version manifest, version JSON, asset
//! index). Field names follow the upstream JSON.
use serde::Deserialize;
use std::collections::HashMap;

// ─── Mojang API types ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct VersionManifest {
    pub(super) versions: Vec<ManifestEntry>,
}

#[derive(Deserialize)]
pub(super) struct ManifestEntry {
    pub(super) id: String,
    pub(super) url: String,
}

#[derive(Deserialize)]
pub(super) struct VersionJson {
    #[serde(rename = "mainClass")]
    pub(super) main_class: String,
    // Pre-1.13 style
    #[serde(rename = "minecraftArguments", default)]
    pub(super) minecraft_arguments: Option<String>,
    // 1.13+ style
    #[serde(default)]
    pub(super) arguments: Option<NewArguments>,
    #[serde(rename = "assetIndex")]
    pub(super) asset_index: AssetIndexRef,
    pub(super) downloads: ClientDownloads,
    pub(super) libraries: Vec<Library>,
    #[serde(rename = "javaVersion", default)]
    pub(super) java_version: Option<JavaVersionReq>,
}

#[derive(Deserialize)]
pub(super) struct NewArguments {
    #[serde(default)]
    pub(super) game: Vec<Arg>,
    #[serde(default)]
    pub(super) jvm: Vec<Arg>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Arg {
    Plain(String),
    Conditional { rules: Vec<ArgRule>, value: ArgValue },
}

#[derive(Deserialize, Clone)]
pub(super) struct ArgRule {
    pub(super) action: String,
    #[serde(default)]
    pub(super) os: Option<OsCondition>,
    #[serde(default)]
    pub(super) features: Option<HashMap<String, bool>>,
}

#[derive(Deserialize, Clone)]
pub(super) struct OsCondition {
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) arch: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum ArgValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize)]
pub(super) struct AssetIndexRef {
    pub(super) id: String,
    pub(super) url: String,
}

#[derive(Deserialize)]
pub(super) struct ClientDownloads {
    pub(super) client: FileRef,
}

#[derive(Deserialize)]
pub(super) struct FileRef {
    pub(super) url: String,
    pub(super) sha1: String,
    pub(super) size: u64,
}

#[derive(Deserialize)]
pub(super) struct Library {
    pub(super) name: String,
    #[serde(default)]
    pub(super) downloads: Option<LibDownloads>,
    #[serde(default)]
    pub(super) rules: Vec<ArgRule>,
    #[serde(default)]
    pub(super) natives: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
pub(super) struct LibDownloads {
    #[serde(default)]
    pub(super) artifact: Option<Artifact>,
    #[serde(default)]
    pub(super) classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Deserialize)]
pub(super) struct Artifact {
    pub(super) path: String,
    pub(super) url: String,
    pub(super) sha1: String,
    pub(super) size: u64,
}

#[derive(Deserialize)]
pub struct JavaVersionReq {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Deserialize)]
pub(super) struct AssetIndex {
    pub(super) objects: HashMap<String, AssetObj>,
    // True for pre-1.7.3 indexes: every object must also be reachable at
    // assets/virtual/legacy/<index key>, or old versions start without textures.
    #[serde(rename = "map_to_resources", default)]
    pub(super) map_to_resources: bool,
}

#[derive(Deserialize)]
pub(super) struct AssetObj {
    pub(super) hash: String,
    pub(super) size: u64,
}
