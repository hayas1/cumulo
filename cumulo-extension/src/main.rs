mod clip;
mod mode;
mod popup;

fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    mode::Mode::current().mount();
}
