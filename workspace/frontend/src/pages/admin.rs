use yew::prelude::*;
use gloo_net::http::Request;
use common::{ApiResponse, AccountDto, CreateAccountRequest, UpdateAccountRequest, TagDto, CreateTagRequest, UpdateTagRequest};

/// Admin page with tabs to manage Accounts and Tags
#[function_component(Admin)]
pub fn admin() -> Html {
    let tab = use_state(|| "accounts".to_string());

    let on_accounts = {
        let tab = tab.clone();
        Callback::from(move |_| tab.set("accounts".into()))
    };
    let on_tags = {
        let tab = tab.clone();
        Callback::from(move |_| tab.set("tags".into()))
    };

    html! {
        <div class="space-y-6">
            <div role="tablist" class="tabs tabs-bordered">
                <a role="tab" class={classes!("tab", if *tab == "accounts" { "tab-active" } else { "" })} onclick={on_accounts}>{"Accounts"}</a>
                <a role="tab" class={classes!("tab", if *tab == "tags" { "tab-active" } else { "" })} onclick={on_tags}>{"Tags"}</a>
            </div>
            if *tab == "accounts" { <AccountsPanel /> } else { <TagsPanel /> }
        </div>
    }
}

// ===================== Accounts Panel =====================

#[function_component(AccountsPanel)]
fn accounts_panel() -> Html {
    let accounts = use_state(|| Vec::<AccountDto>::new());
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);

    {
        let accounts = accounts.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let resp = Request::get("/api/v1/accounts").send().await;
                match resp {
                    Ok(r) => match r.json::<ApiResponse<Vec<AccountDto>>>().await {
                        Ok(body) => {
                            accounts.set(body.data);
                            loading.set(false);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to parse accounts: {}", e)));
                            loading.set(false);
                        }
                    },
                    Err(e) => {
                        error.set(Some(format!("Failed to load accounts: {}", e)));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }

    html! {
        <div class="space-y-4">
            <AccountForm on_created={{
                let accounts = accounts.clone();
                Callback::from(move |a: AccountDto| {
                    let mut v = (*accounts).clone();
                    v.push(a);
                    accounts.set(v);
                })
            }} />

            if *loading { <div class="alert">{"Loading accounts..."}</div> }
            if let Some(err) = &*error { <div class="alert alert-error">{err.clone()}</div> }
            <div class="overflow-x-auto">
                <table class="table">
                    <thead>
                        <tr>
                            <th>{"ID"}</th>
                            <th>{"Name"}</th>
                            <th>{"Currency"}</th>
                            <th>{"Owner"}</th>
                            <th>{"Include"}</th>
                            <th>{"Actions"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for accounts.iter().cloned().map(|a| html!{ <AccountRow account={a} on_updated={{
                            let accounts = accounts.clone();
                            Callback::from(move |updated: AccountDto| {
                                let mut v = (*accounts).clone();
                                if let Some(pos) = v.iter().position(|x| x.id == updated.id) { v[pos] = updated; }
                                accounts.set(v);
                            })
                        }} on_deleted={{
                            let accounts = accounts.clone();
                            Callback::from(move |id: i32| {
                                let v = (*accounts).clone().into_iter().filter(|x| x.id != id).collect();
                                accounts.set(v);
                            })
                        }} /> }) }
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct AccountFormProps {
    pub on_created: Callback<AccountDto>,
}

#[function_component(AccountForm)]
fn account_form(props: &AccountFormProps) -> Html {
    let name = use_state(|| String::new());
    let currency = use_state(|| "USD".to_string());
    let owner_id = use_state(|| String::new());
    let description = use_state(|| String::new());
    let include = use_state(|| true);
    let ledger_name = use_state(|| String::new());
    let busy = use_state(|| false);
    let error = use_state(|| None::<String>);

    let on_submit = {
        let name = name.clone();
        let currency = currency.clone();
        let owner_id = owner_id.clone();
        let description = description.clone();
        let include = include.clone();
        let ledger_name = ledger_name.clone();
        let busy = busy.clone();
        let error = error.clone();
        let on_created = props.on_created.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if *busy { return; }
            busy.set(true);
            error.set(None);
            let req = CreateAccountRequest {
                name: (*name).clone(),
                description: if description.is_empty() { None } else { Some((*description).clone()) },
                currency_code: (*currency).clone(),
                owner_id: owner_id.parse::<i32>().unwrap_or(1),
                include_in_statistics: Some(*include),
                ledger_name: if ledger_name.is_empty() { None } else { Some((*ledger_name).clone()) },
            };
            wasm_bindgen_futures::spawn_local(async move {
                let resp = Request::post("/api/v1/accounts").json(&req).unwrap().send().await;
                match resp {
                    Ok(r) => match r.json::<ApiResponse<AccountDto>>().await {
                        Ok(body) => {
                            on_created.emit(body.data);
                        }
                        Err(e) => error.set(Some(format!("Failed to create account: {}", e))),
                    },
                    Err(e) => error.set(Some(format!("Failed to create account: {}", e))),
                }
                busy.set(false);
            });
        })
    };

    html! {
        <form class="card bg-base-200 p-4 space-y-2" onsubmit={on_submit}>
            <div class="font-semibold">{"Create Account"}</div>
            if let Some(err) = &*error { <div class="alert alert-error">{err.clone()}</div> }
            <div class="grid grid-cols-1 md:grid-cols-3 gap-2">
                <input class="input input-bordered" placeholder="Name" value={(*name).clone()} oninput={{
                    let name = name.clone(); Callback::from(move |e: web_sys::InputEvent| {
                        let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                        name.set(v);
                    })
                }} />
                <input class="input input-bordered" placeholder="Currency (e.g. USD)" value={(*currency).clone()} oninput={{
                    let currency = currency.clone(); Callback::from(move |e: web_sys::InputEvent| {
                        let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                        currency.set(v);
                    })
                }} />
                <input class="input input-bordered" placeholder="Owner ID" value={(*owner_id).clone()} oninput={{
                    let owner_id = owner_id.clone(); Callback::from(move |e: web_sys::InputEvent| {
                        let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                        owner_id.set(v);
                    })
                }} />
                <input class="input input-bordered md:col-span-2" placeholder="Description" value={(*description).clone()} oninput={{
                    let description = description.clone(); Callback::from(move |e: web_sys::InputEvent| {
                        let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                        description.set(v);
                    })
                }} />
                <input class="input input-bordered" placeholder="Ledger name" value={(*ledger_name).clone()} oninput={{
                    let ledger_name = ledger_name.clone(); Callback::from(move |e: web_sys::InputEvent| {
                        let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                        ledger_name.set(v);
                    })
                }} />
            </div>
            <label class="label cursor-pointer w-fit gap-2">
                <input type="checkbox" class="checkbox" checked={*include} onchange={{
                    let include = include.clone(); Callback::from(move |e: SubmitEvent| {
                        let checked = e.target_unchecked_into::<web_sys::HtmlInputElement>().checked();
                        include.set(checked);
                    })
                }} />
                <span class="label-text">{"Include in statistics"}</span>
            </label>
            <button class={classes!("btn btn-primary", if *busy {"btn-disabled"} else {""})} type="submit">{"Create"}</button>
        </form>
    }
}

#[derive(Properties, PartialEq)]
struct AccountRowProps {
    pub account: AccountDto,
    pub on_updated: Callback<AccountDto>,
    pub on_deleted: Callback<i32>,
}

#[function_component(AccountRow)]
fn account_row(props: &AccountRowProps) -> Html {
    let editing = use_state(|| false);
    let name = use_state(|| props.account.name.clone());
    let description = use_state(|| props.account.description.clone().unwrap_or_default());
    let busy = use_state(|| false);

    let on_save = {
        let id = props.account.id;
        let name = name.clone();
        let description = description.clone();
        let on_updated = props.on_updated.clone();
        let busy = busy.clone();
        let editing = editing.clone();
        Callback::from(move |_| {
            if *busy { return; }
            busy.set(true);
            let req = UpdateAccountRequest { name: Some((*name).clone()), description: Some((*description).clone()), ..Default::default() };
            wasm_bindgen_futures::spawn_local(async move {
                let resp = Request::put(&format!("/api/v1/accounts/{}", id)).json(&req).unwrap().send().await;
                if let Ok(r) = resp { if let Ok(body) = r.json::<ApiResponse<AccountDto>>().await { on_updated.emit(body.data); } }
                busy.set(false);
                editing.set(false);
            });
        })
    };

    let on_delete = {
        let id = props.account.id;
        let on_deleted = props.on_deleted.clone();
        Callback::from(move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = Request::delete(&format!("/api/v1/accounts/{}", id)).send().await;
                on_deleted.emit(id);
            });
        })
    };

    html! {
        <tr>
            <td>{ props.account.id }</td>
            <td>
                if *editing {
                    <input class="input input-bordered" value={(*name).clone()} oninput={{
                        let name = name.clone(); Callback::from(move |e: web_sys::InputEvent| {
                            let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                            name.set(v);
                        })
                    }} />
                } else {
                    { props.account.name.clone() }
                }
            </td>
            <td>{ props.account.currency_code.clone() }</td>
            <td>{ props.account.owner_id }</td>
            <td>{ if props.account.include_in_statistics { "✔" } else { "✖" } }</td>
            <td class="space-x-2">
                if *editing {
                    <button class={classes!("btn btn-sm btn-primary", if *busy {"btn-disabled"} else {""})} onclick={on_save.clone()}>{"Save"}</button>
                    <button class="btn btn-sm" onclick={{ let editing = editing.clone(); Callback::from(move |_| editing.set(false)) }}>{"Cancel"}</button>
                } else {
                    <button class="btn btn-sm" onclick={{ let editing = editing.clone(); Callback::from(move |_| editing.set(true)) }}>{"Edit"}</button>
                    <button class="btn btn-sm btn-error" onclick={on_delete}>{"Delete"}</button>
                }
            </td>
        </tr>
    }
}

// ===================== Tags Panel =====================

#[function_component(TagsPanel)]
fn tags_panel() -> Html {
    let tags = use_state(|| Vec::<TagDto>::new());
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);

    {
        let tags = tags.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let resp = Request::get("/api/v1/tags").send().await;
                match resp {
                    Ok(r) => match r.json::<ApiResponse<Vec<TagDto>>>().await {
                        Ok(body) => { tags.set(body.data); loading.set(false); }
                        Err(e) => { error.set(Some(format!("Failed to parse tags: {}", e))); loading.set(false); }
                    },
                    Err(e) => { error.set(Some(format!("Failed to load tags: {}", e))); loading.set(false); }
                }
            });
            || ()
        });
    }

    html! {
        <div class="space-y-4">
            <TagForm on_created={{
                let tags = tags.clone();
                Callback::from(move |t: TagDto| {
                    let mut v = (*tags).clone(); v.push(t); tags.set(v);
                })
            }} />

            if *loading { <div class="alert">{"Loading tags..."}</div> }
            if let Some(err) = &*error { <div class="alert alert-error">{err.clone()}</div> }

            <div class="overflow-x-auto">
                <table class="table">
                    <thead>
                        <tr>
                            <th>{"ID"}</th>
                            <th>{"Name"}</th>
                            <th>{"Parent"}</th>
                            <th>{"Actions"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for tags.iter().cloned().map(|t| html!{ <TagRow tag={t} on_updated={{
                            let tags = tags.clone(); Callback::from(move |updated: TagDto| {
                                let mut v = (*tags).clone(); if let Some(pos) = v.iter().position(|x| x.id == updated.id) { v[pos] = updated; } tags.set(v);
                            })
                        }} on_deleted={{
                            let tags = tags.clone(); Callback::from(move |id: i32| {
                                let v = (*tags).clone().into_iter().filter(|x| x.id != id).collect(); tags.set(v);
                            })
                        }} /> }) }
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct TagFormProps { pub on_created: Callback<TagDto> }

#[function_component(TagForm)]
fn tag_form(props: &TagFormProps) -> Html {
    let name = use_state(|| String::new());
    let description = use_state(|| String::new());
    let parent_id = use_state(|| String::new());
    let ledger_name = use_state(|| String::new());
    let busy = use_state(|| false);

    let on_submit = {
        let name = name.clone(); let description = description.clone(); let parent_id = parent_id.clone(); let ledger_name = ledger_name.clone();
        let on_created = props.on_created.clone(); let busy = busy.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default(); if *busy { return; } busy.set(true);
            let req = CreateTagRequest {
                name: (*name).clone(),
                description: if description.is_empty() { None } else { Some((*description).clone()) },
                parent_id: parent_id.parse::<i32>().ok(),
                ledger_name: if ledger_name.is_empty() { None } else { Some((*ledger_name).clone()) },
            };
            wasm_bindgen_futures::spawn_local(async move {
                let resp = Request::post("/api/v1/tags").json(&req).unwrap().send().await;
                if let Ok(r) = resp { if let Ok(body) = r.json::<ApiResponse<TagDto>>().await { on_created.emit(body.data); } }
                busy.set(false);
            });
        })
    };

    html! {
        <form class="card bg-base-200 p-4 space-y-2" onsubmit={on_submit}>
            <div class="font-semibold">{"Create Tag"}</div>
            <div class="grid grid-cols-1 md:grid-cols-4 gap-2">
                <input class="input input-bordered" placeholder="Name" value={(*name).clone()} oninput={{
                    let name = name.clone(); Callback::from(move |e: web_sys::InputEvent| {
                        let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value(); name.set(v);
                    })
                }} />
                <input class="input input-bordered md:col-span-2" placeholder="Description" value={(*description).clone()} oninput={{
                    let description = description.clone(); Callback::from(move |e: web_sys::InputEvent| {
                        let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value(); description.set(v);
                    })
                }} />
                <input class="input input-bordered" placeholder="Parent ID" value={(*parent_id).clone()} oninput={{
                    let parent_id = parent_id.clone(); Callback::from(move |e: web_sys::InputEvent| {
                        let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value(); parent_id.set(v);
                    })
                }} />
                <input class="input input-bordered md:col-span-2" placeholder="Ledger name" value={(*ledger_name).clone()} oninput={{
                    let ledger_name = ledger_name.clone(); Callback::from(move |e: web_sys::InputEvent| {
                        let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value(); ledger_name.set(v);
                    })
                }} />
            </div>
            <button class={classes!("btn btn-primary", if *busy {"btn-disabled"} else {""})} type="submit">{"Create"}</button>
        </form>
    }
}

#[derive(Properties, PartialEq)]
struct TagRowProps { pub tag: TagDto, pub on_updated: Callback<TagDto>, pub on_deleted: Callback<i32> }

#[function_component(TagRow)]
fn tag_row(props: &TagRowProps) -> Html {
    let editing = use_state(|| false);
    let name = use_state(|| props.tag.name.clone());
    let busy = use_state(|| false);

    let on_save = {
        let id = props.tag.id; let name = name.clone(); let on_updated = props.on_updated.clone(); let busy = busy.clone(); let editing = editing.clone();
        Callback::from(move |_| {
            if *busy { return; } busy.set(true);
            let req = UpdateTagRequest { name: Some((*name).clone()), ..Default::default() };
            wasm_bindgen_futures::spawn_local(async move {
                let resp = Request::put(&format!("/api/v1/tags/{}", id)).json(&req).unwrap().send().await;
                if let Ok(r) = resp { if let Ok(body) = r.json::<ApiResponse<TagDto>>().await { on_updated.emit(body.data); } }
                busy.set(false); editing.set(false);
            });
        })
    };

    let on_delete = {
        let id = props.tag.id; let on_deleted = props.on_deleted.clone();
        Callback::from(move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = Request::delete(&format!("/api/v1/tags/{}", id)).send().await; on_deleted.emit(id);
            });
        })
    };

    html! {
        <tr>
            <td>{ props.tag.id }</td>
            <td>
                if *editing {
                    <input class="input input-bordered" value={(*name).clone()} oninput={{
                        let name = name.clone(); Callback::from(move |e: web_sys::InputEvent| {
                            let v = e.target_unchecked_into::<web_sys::HtmlInputElement>().value(); name.set(v);
                        })
                    }} />
                } else { { props.tag.name.clone() } }
            </td>
            <td>{ props.tag.parent_id.map(|x| x.to_string()).unwrap_or_else(|| "-".to_string()) }</td>
            <td class="space-x-2">
                if *editing {
                    <button class={classes!("btn btn-sm btn-primary", if *busy {"btn-disabled"} else {""})} onclick={on_save}>{"Save"}</button>
                    <button class="btn btn-sm" onclick={{ let editing = editing.clone(); Callback::from(move |_| editing.set(false)) }}>{"Cancel"}</button>
                } else {
                    <button class="btn btn-sm" onclick={{ let editing = editing.clone(); Callback::from(move |_| editing.set(true)) }}>{"Edit"}</button>
                    <button class="btn btn-sm btn-error" onclick={on_delete}>{"Delete"}</button>
                }
            </td>
        </tr>
    }
}
