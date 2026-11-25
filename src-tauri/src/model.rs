use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CliOutput {
    pub success: bool,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionItem {
    pub version: String,
    pub active: bool,
    pub path: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnvStatus {
    pub tool_path: Option<String>,
    pub current: Option<String>,
    pub link_path: Option<String>,
    pub link_target: Option<String>,
    pub probe: Option<String>,
    pub consistent: bool,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionPage {
    pub status: EnvStatus,
    pub versions: Vec<VersionItem>,
    pub error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoteVersion {
    pub version: String,
    pub group: Option<String>,
    pub installed: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PortItem {
    pub port: u16,
    pub address: String,
    pub pid: u32,
    pub process_name: String,
    pub exe_path: Option<String>,
    pub cmdline: Option<String>,
    pub start_time: Option<u64>,
    pub kind: String,
    pub protected: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KillResult {
    pub success: bool,
    pub message: String,
    pub released: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppEnv {
    pub is_admin: bool,
    pub jvms_path: Option<String>,
    pub nvm_path: Option<String>,
    pub java_home: Option<String>,
    pub nvm_symlink: Option<String>,
}
