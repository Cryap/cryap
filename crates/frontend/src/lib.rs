pub mod components;

#[cfg(feature = "ssr")]
pub mod ssr {
    use leptos::{provide_context, ssr::render_to_string_async, view};
    use leptos_router::{RouterIntegrationContext, ServerIntegration};

    use crate::components::app::App;
    pub async fn render(path: String) -> String {
        render_to_string_async(|| {
            let integration = ServerIntegration { path };
            provide_context(RouterIntegrationContext::new(integration));
            view! { <App /> }
        })
        .await
    }
}
