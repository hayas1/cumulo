use cumulo_web::RootLocalStore;
use leptos::prelude::*;

fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mount_to_body(RootLocalStore);
}
