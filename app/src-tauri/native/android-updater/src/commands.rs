use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime, command};

use crate::AndroidUpdaterExt;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub available: bool,
    pub requires_reinstall: bool,
    pub version: Option<String>,
    pub version_code: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloading: bool,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub permission_required: bool,
    pub installer_launched: bool,
}

#[command]
pub(crate) async fn check_update<R: Runtime>(
    app: AppHandle<R>,
) -> Result<UpdateCheck, String> {
    app.android_updater().call("checkUpdate")
}

#[command]
pub(crate) async fn download_update<R: Runtime>(
    app: AppHandle<R>,
) -> Result<serde_json::Value, String> {
    app.android_updater().call("downloadUpdate")
}

#[command]
pub(crate) async fn get_download_progress<R: Runtime>(
    app: AppHandle<R>,
) -> Result<DownloadProgress, String> {
    app.android_updater().call("getDownloadProgress")
}

#[command]
pub(crate) async fn install_update<R: Runtime>(
    app: AppHandle<R>,
) -> Result<InstallResult, String> {
    app.android_updater().call("installUpdate")
}

#[command]
pub(crate) async fn open_release_page<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    app.android_updater().call("openReleasePage")
}
