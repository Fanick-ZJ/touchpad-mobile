mod app;

use app::main::Main;
use leptos::mount;

fn main() {
    console_error_panic_hook::set_once();
    mount::mount_to_body(Main);
}
