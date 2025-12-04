use yew::prelude::*;

#[function_component(RecentActivity)]
pub fn recent_activity() -> Html {
    html! {
        <div class="overflow-x-auto">
            <table class="table table-sm">
                <thead>
                    <tr>
                        <th>{"Date"}</th>
                        <th>{"Description"}</th>
                        <th>{"Amount"}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>{"2023-10-01"}</td>
                        <td>{"Salary"}</td>
                        <td class="text-success">{"$3,500.00"}</td>
                    </tr>
                    <tr>
                        <td>{"2023-10-02"}</td>
                        <td>{"Rent"}</td>
                        <td class="text-error">{"-$1,200.00"}</td>
                    </tr>
                    <tr>
                        <td>{"2023-10-05"}</td>
                        <td>{"Groceries"}</td>
                        <td class="text-error">{"-$150.00"}</td>
                    </tr>
                    <tr>
                        <td>{"2023-10-08"}</td>
                        <td>{"Utility Bill"}</td>
                        <td class="text-error">{"-$85.00"}</td>
                    </tr>
                </tbody>
            </table>
        </div>
    }
}
