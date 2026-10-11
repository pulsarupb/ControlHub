const COMMANDS: &[&str] = &[
    "check_update",
    "download_update",
    "get_download_progress",
    "install_update",
    "open_release_page",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
