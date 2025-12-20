use tauri_sys::core::invoke;

pub async fn invoke_no_args<T>(cmd: impl Into<String>) -> T
where
    T: serde::de::DeserializeOwned,
{
    let cmd = cmd.into();
    invoke(cmd.as_str(), &()).await
}
