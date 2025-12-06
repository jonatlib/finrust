use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ErrorDisplayProps {
    pub message: String,
    #[prop_or_default]
    pub on_retry: Option<Callback<()>>,
}

#[function_component(ErrorDisplay)]
pub fn error_display(props: &ErrorDisplayProps) -> Html {
    log::warn!("Displaying error to user: {}", props.message);

    html! {
        <div class="flex flex-col items-center justify-center py-12 gap-4">
            <div class="alert alert-error max-w-lg">
                <i class="fas fa-exclamation-circle text-2xl"></i>
                <div class="flex flex-col gap-2">
                    <span class="font-semibold">{"Something went wrong"}</span>
                    <span class="text-sm">{&props.message}</span>
                </div>
            </div>
            {if let Some(on_retry) = &props.on_retry {
                let on_retry = on_retry.clone();
                html! {
                    <button
                        class="btn btn-primary btn-sm"
                        onclick={Callback::from(move |_| {
                            log::debug!("User clicked retry button");
                            on_retry.emit(());
                        })}
                    >
                        <i class="fas fa-redo"></i>
                        {" Try Again"}
                    </button>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
