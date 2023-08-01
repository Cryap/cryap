use leptos::view;
use frontend::components::app::App;
pub fn main() {
    leptos::mount_to_body(move |cx| {
        view! { cx, <App/> }
    });
}
