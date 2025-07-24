use yew::prelude::*;

#[function_component(About)]
pub fn about() -> Html {
    html! {
        <div class="container mx-auto px-4 py-8">
            <div class="card bg-base-100 shadow-xl">
                <div class="card-body">
                    <h2 class="card-title text-3xl mb-6">{"About FinRust"}</h2>
                    
                    <div class="prose max-w-none">
                        <p class="text-lg mb-4">
                            {"FinRust is a modern financial management application built with cutting-edge web technologies."}
                        </p>
                        
                        <h3 class="text-xl font-semibold mb-3">{"Technology Stack"}</h3>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
                            <div class="badge badge-primary badge-lg">{"Rust"}</div>
                            <div class="badge badge-secondary badge-lg">{"Yew Framework"}</div>
                            <div class="badge badge-accent badge-lg">{"WebAssembly"}</div>
                            <div class="badge badge-neutral badge-lg">{"Tailwind CSS"}</div>
                            <div class="badge badge-info badge-lg">{"Daisy UI"}</div>
                            <div class="badge badge-success badge-lg">{"Axum Backend"}</div>
                        </div>
                        
                        <h3 class="text-xl font-semibold mb-3">{"Features"}</h3>
                        <ul class="list-disc list-inside space-y-2">
                            <li>{"Account management and tracking"}</li>
                            <li>{"Transaction categorization and analysis"}</li>
                            <li>{"Financial reporting and insights"}</li>
                            <li>{"Modern, responsive user interface"}</li>
                            <li>{"Real-time data synchronization"}</li>
                        </ul>
                    </div>
                    
                    <div class="card-actions justify-end mt-6">
                        <button class="btn btn-primary">{"Get Started"}</button>
                    </div>
                </div>
            </div>
        </div>
    }
}