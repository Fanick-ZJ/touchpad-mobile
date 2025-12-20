use super::invoke::invoke_no_args;
pub async fn start_discover_service() {
    // Implementation of start_discover_service
    invoke_no_args("start_discover_service").await
}

pub async fn get_language() -> String {
    invoke_no_args::<String>("get_language").await
}
