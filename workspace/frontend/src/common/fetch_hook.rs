use yew::prelude::*;
use std::future::Future;
use std::rc::Rc;
use crate::hooks::FetchState;
use crate::common::toast::ToastContext;

#[hook]
pub fn use_fetch_with_refetch<T, F, Fut>(fetch_fn: F) -> (UseStateHandle<FetchState<T>>, Callback<()>)
where
    T: 'static,
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, String>> + 'static,
{
    let fetch_state = use_state(|| FetchState::Loading);
    let toast_ctx = use_context::<ToastContext>().unwrap();
    let fetch_fn = use_state(|| Rc::new(fetch_fn));

    let refetch = {
        let fetch_state = fetch_state.clone();
        let toast_ctx = toast_ctx.clone();
        let fetch_fn = fetch_fn.clone();

        use_callback((), move |_, _| {
            let fetch_state = fetch_state.clone();
            let toast_ctx = toast_ctx.clone();
            let fetch_fn = fetch_fn.clone();

            fetch_state.set(FetchState::Loading);

            wasm_bindgen_futures::spawn_local(async move {
                let fut = (*fetch_fn)();
                match fut.await {
                    Ok(data) => fetch_state.set(FetchState::Success(data)),
                    Err(err) => {
                        fetch_state.set(FetchState::Error(err.clone()));
                        toast_ctx.show_error(err);
                    }
                }
            });
        })
    };

    // Fetch on mount
    {
        let refetch = refetch.clone();
        use_effect_with((), move |_| {
            refetch.emit(());
            || ()
        });
    }

    (fetch_state, refetch)
}
