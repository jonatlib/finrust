use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ToastType {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastType {
    fn alert_class(&self) -> &'static str {
        match self {
            ToastType::Info => "alert-info",
            ToastType::Success => "alert-success",
            ToastType::Warning => "alert-warning",
            ToastType::Error => "alert-error",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            ToastType::Info => "fas fa-info-circle",
            ToastType::Success => "fas fa-check-circle",
            ToastType::Warning => "fas fa-exclamation-triangle",
            ToastType::Error => "fas fa-exclamation-circle",
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Toast {
    pub id: usize,
    pub message: String,
    pub toast_type: ToastType,
}

#[derive(Clone, PartialEq)]
pub struct ToastContext {
    pub toasts: Vec<Toast>,
    pub add_toast: Callback<(String, ToastType)>,
    pub remove_toast: Callback<usize>,
}

impl ToastContext {
    pub fn show_info(&self, message: String) {
        self.add_toast.emit((message, ToastType::Info));
    }

    pub fn show_success(&self, message: String) {
        self.add_toast.emit((message, ToastType::Success));
    }

    pub fn show_warning(&self, message: String) {
        self.add_toast.emit((message, ToastType::Warning));
    }

    pub fn show_error(&self, message: String) {
        self.add_toast.emit((message, ToastType::Error));
    }
}

#[derive(Properties, PartialEq)]
pub struct ToastProviderProps {
    pub children: Children,
}

#[function_component(ToastProvider)]
pub fn toast_provider(props: &ToastProviderProps) -> Html {
    let toasts = use_state(|| Vec::<Toast>::new());
    let next_id = use_state(|| 0usize);

    let add_toast = {
        let toasts = toasts.clone();
        let next_id = next_id.clone();

        Callback::from(move |(message, toast_type): (String, ToastType)| {
            let id = *next_id;
            next_id.set(id + 1);

            let mut new_toasts = (*toasts).clone();
            new_toasts.push(Toast {
                id,
                message,
                toast_type,
            });
            toasts.set(new_toasts);

            // Auto-dismiss after 5 seconds
            let toasts_clone = toasts.clone();
            let timeout_handle = gloo_timers::callback::Timeout::new(5000, move || {
                let mut new_toasts = (*toasts_clone).clone();
                new_toasts.retain(|t| t.id != id);
                toasts_clone.set(new_toasts);
            });
            timeout_handle.forget();
        })
    };

    let remove_toast = {
        let toasts = toasts.clone();

        Callback::from(move |id: usize| {
            let mut new_toasts = (*toasts).clone();
            new_toasts.retain(|t| t.id != id);
            toasts.set(new_toasts);
        })
    };

    let context = ToastContext {
        toasts: (*toasts).clone(),
        add_toast,
        remove_toast: remove_toast.clone(),
    };

    html! {
        <ContextProvider<ToastContext> context={context}>
            {props.children.clone()}
            <div class="toast toast-top toast-end z-50">
                {for (*toasts).iter().map(|toast| {
                    let id = toast.id;
                    let on_close = {
                        let remove_toast = remove_toast.clone();
                        Callback::from(move |_| remove_toast.emit(id))
                    };

                    html! {
                        <div key={id} class={classes!("alert", toast.toast_type.alert_class(), "shadow-lg")}>
                            <i class={toast.toast_type.icon()}></i>
                            <span>{&toast.message}</span>
                            <button class="btn btn-sm btn-ghost btn-circle" onclick={on_close}>
                                <i class="fas fa-times"></i>
                            </button>
                        </div>
                    }
                })}
            </div>
        </ContextProvider<ToastContext>>
    }
}

