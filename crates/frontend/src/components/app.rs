use leptos::component;use leptos::view;

use leptos::Scope;
use leptos::IntoView;
use leptos::create_signal;
use leptos::SignalUpdate;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(cx, 0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    view! { cx,
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}
