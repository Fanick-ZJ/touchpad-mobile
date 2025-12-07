mod app;

use app::*;
use leptos::mount;

fn main() {
    console_error_panic_hook::set_once();
    mount::mount_to_body(App);
}
