use gethostname::gethostname;
pub fn get_computer_name() -> String {
    { gethostname().to_string_lossy().to_string() }
}
