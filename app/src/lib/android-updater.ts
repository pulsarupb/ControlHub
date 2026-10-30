import { invoke } from "@tauri-apps/api/core"

export interface AndroidUpdateCheck {
  available: boolean
  requiresReinstall: boolean
  version?: string
  versionCode?: number
  notes?: string
  releasePageUrl?: string
}

export interface AndroidDownloadProgress {
  downloading: boolean
  downloadedBytes: number
  totalBytes: number
}

export interface AndroidInstallResult {
  permissionRequired: boolean
  installerLaunched: boolean
}

export const androidUpdater = {
  check: () => invoke<AndroidUpdateCheck>("plugin:android-updater|check_update"),
  download: () => invoke<{ downloaded: boolean }>("plugin:android-updater|download_update"),
  progress: () => invoke<AndroidDownloadProgress>("plugin:android-updater|get_download_progress"),
  install: () => invoke<AndroidInstallResult>("plugin:android-updater|install_update"),
  openReleasePage: () => invoke<void>("plugin:android-updater|open_release_page"),
}
