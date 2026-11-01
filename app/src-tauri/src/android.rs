//! Android-only bridge to the Kotlin `MoonlockPlugin`
//! (gen/android/app/src/main/java/io/github/thanhphu2643/moonlock/MoonlockPlugin.kt):
//! calendar sync, Downloads export and the native bottom navigation, i.e. what
//! iOS gets from swift/MoonlockKit.

use serde::{de::DeserializeOwned, Serialize};
use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    AppHandle, Manager, Wry,
};

struct Bridge(PluginHandle<Wry>);

pub fn init() -> TauriPlugin<Wry> {
    Builder::new("moonlock")
        .setup(|app, api| {
            let handle = api.register_android_plugin("io.github.thanhphu2643.moonlock", "MoonlockPlugin")?;
            app.manage(Bridge(handle));
            Ok(())
        })
        .build()
}

/// Blocks until Kotlin resolves, so never call this on the Android UI thread
/// (use it from async commands or a spawned thread).
pub fn call<T: DeserializeOwned>(app: &AppHandle, cmd: &str, payload: impl Serialize) -> Result<T, String> {
    use tauri::plugin::mobile::PluginInvokeError as E;
    app.state::<Bridge>().0.run_mobile_plugin(cmd, payload).map_err(|e| match e {
        E::InvokeRejected(r) => r.message.unwrap_or_else(|| "Lỗi không rõ.".into()),
        e => e.to_string(),
    })
}
