use yew::prelude::*;

#[function_component(Home)]
pub fn home() -> Html {
    html! {
        <div class="hero min-h-screen bg-base-200">
            <div class="hero-content text-center">
                <div class="max-w-md">
                    <h1 class="text-5xl font-bold">{"Welcome to FinRust"}</h1>
                    <p class="py-6">
                        {"A modern financial management application built with Rust and Yew. "}
                        {"Track your finances, manage accounts, and analyze your spending patterns."}
                    </p>
                    <div class="flex gap-4 justify-center">
                        <button class="btn btn-primary">{"Get Started"}</button>
                        <button class="btn btn-outline">{"Learn More"}</button>
                    </div>
                </div>
            </div>
        </div>
    }
}