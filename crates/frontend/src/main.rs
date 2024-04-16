use frontend::components::app::App;
use leptos::view;

pub fn main() {
    leptos::mount_to_body(|| {
        view! { <App/> }
    });
}
