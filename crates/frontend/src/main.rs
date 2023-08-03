use frontend::components::app::App;
use leptos::view;
pub fn main() {
    leptos::mount_to_body(move |cx| {
        view! { cx, <App/> }
    });
}
