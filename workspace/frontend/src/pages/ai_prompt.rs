use yew::prelude::*;

use crate::components::ai_prompt::AiPrompt;
use crate::components::layout::layout::Layout;

#[function_component(AiPromptPage)]
pub fn ai_prompt_page() -> Html {
    html! {
        <Layout title="AI Financial Assessment">
            <AiPrompt />
        </Layout>
    }
}
