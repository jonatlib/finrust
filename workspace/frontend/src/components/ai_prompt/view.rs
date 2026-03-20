use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::api_client;

#[function_component(AiPrompt)]
pub fn ai_prompt() -> Html {
    let prompt_text = use_state(|| None::<String>);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let copied = use_state(|| false);

    {
        let prompt_text = prompt_text.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                match api_client::get::<String>("/prompt?months=24").await {
                    Ok(text) => {
                        prompt_text.set(Some(text));
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }

    let on_copy = {
        let prompt_text = prompt_text.clone();
        let copied = copied.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(text) = (*prompt_text).as_ref() {
                let text = text.clone();
                let copied = copied.clone();
                spawn_local(async move {
                    let window = web_sys::window().unwrap();
                    let nav: js_sys::Object = js_sys::Reflect::get(&window, &"navigator".into())
                        .unwrap()
                        .unchecked_into();
                    let clip: js_sys::Object = js_sys::Reflect::get(&nav, &"clipboard".into())
                        .unwrap()
                        .unchecked_into();
                    let func: js_sys::Function =
                        js_sys::Reflect::get(&clip, &"writeText".into())
                            .unwrap()
                            .unchecked_into();
                    let promise: js_sys::Promise = func
                        .call1(&clip, &text.into())
                        .unwrap()
                        .unchecked_into();
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    copied.set(true);
                    gloo_timers::callback::Timeout::new(2000, move || {
                        copied.set(false);
                    })
                    .forget();
                });
            }
        })
    };

    if *loading {
        return html! {
            <div class="flex items-center justify-center min-h-[400px]">
                <span class="loading loading-spinner loading-lg"></span>
                <span class="ml-4 text-lg">{"Generating financial assessment prompt..."}</span>
            </div>
        };
    }

    if let Some(err) = (*error).as_ref() {
        return html! {
            <div class="alert alert-error">
                <i class="fas fa-exclamation-circle"></i>
                <span>{format!("Error: {}", err)}</span>
            </div>
        };
    }

    let text = (*prompt_text).as_ref().cloned().unwrap_or_default();

    html! {
        <div class="space-y-4">
            <div class="flex items-center justify-between">
                <p class="text-sm opacity-70">
                    {"Copy this prompt and paste it into ChatGPT, Claude, or any other LLM for a financial assessment."}
                </p>
                <button
                    class={classes!(
                        "btn",
                        if *copied { "btn-success" } else { "btn-primary" },
                    )}
                    onclick={on_copy}
                >
                    if *copied {
                        <><i class="fas fa-check mr-2"></i>{"Copied!"}</>
                    } else {
                        <><i class="fas fa-copy mr-2"></i>{"Copy to Clipboard"}</>
                    }
                </button>
            </div>
            <div class="card bg-base-100 shadow">
                <div class="card-body p-0">
                    <pre class="whitespace-pre-wrap break-words text-sm p-6 max-h-[75vh] overflow-y-auto font-mono">
                        {text}
                    </pre>
                </div>
            </div>
        </div>
    }
}
