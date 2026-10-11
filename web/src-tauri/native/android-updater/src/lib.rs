use tauri::{Manager, Runtime, plugin::{Builder, TauriPlugin}};

#[cfg(not(target_os = "android"))]
use std::marker::PhantomData;
#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

mod commands;

/// Initializes the Android APK updater plugin. Calls on non-Android targets fail cleanly.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("android-updater")
        .invoke_handler(tauri::generate_handler![
            commands::check_update,
            commands::download_update,
            commands::get_download_progress,
            commands::install_update,
            commands::open_release_page,
        ])
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let updater = AndroidUpdater {
                handle: api.register_android_plugin(
                    "com.bylazar.panels.androidupdater",
                    "AndroidUpdaterPlugin",
                )?,
            };

            #[cfg(not(target_os = "android"))]
            let updater: AndroidUpdater<R> = {
                let _ = api;
                AndroidUpdater {
                    _runtime: PhantomData::<fn() -> R>,
                }
            };

            app.manage(updater);
            Ok(())
        })
        .build()
}

pub(crate) struct AndroidUpdater<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _runtime: PhantomData<fn() -> R>,
}

impl<R: Runtime> AndroidUpdater<R> {
    pub(crate) fn call<T: serde::de::DeserializeOwned>(
        &self,
        command: &str,
    ) -> Result<T, String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin(command, ())
                .map_err(|error| error.to_string())
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = command;
            Err("Android updates are only available on Android".into())
        }
    }
}

pub(crate) trait AndroidUpdaterExt<R: Runtime> {
    fn android_updater(&self) -> &AndroidUpdater<R>;
}

impl<R: Runtime, T: Manager<R>> AndroidUpdaterExt<R> for T {
    fn android_updater(&self) -> &AndroidUpdater<R> {
        self.state::<AndroidUpdater<R>>().inner()
    }
}
