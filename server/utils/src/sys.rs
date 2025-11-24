use std::env;
pub fn get_comptuer_name() -> String {
    // Windwos中
    #[cfg(target_os = "windows")]
    {
        env::var("COMPUTERNAME").unwrap_or_else(|_| "Unknown".to_string())
    }

    #[cfg(unix)]
    {
        env::var("HOSTNAME").unwrap_or_else(|_| "Unknown".to_string())
    }
}
