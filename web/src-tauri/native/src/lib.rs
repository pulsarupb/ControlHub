use tauri::{
    Runtime,
    plugin::{Builder, TauriPlugin},
};

#[cfg(target_os = "android")]
const ANDROID_PLUGIN_PACKAGE: &str = "com.bylazar.panels.systembars";

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("native")
        .setup(|_app, api| {
            #[cfg(target_os = "android")]
            api.register_android_plugin(ANDROID_PLUGIN_PACKAGE, "SystemBarsPlugin")?;

            #[cfg(not(target_os = "android"))]
            let _ = api;

            Ok(())
        })
        .build()
}
