use yew::prelude::*;

#[function_component(TransactionTable)]
pub fn transaction_table() -> Html {
    html! {
        <div class="overflow-x-auto bg-base-100 shadow rounded-box">
            <table class="table">
                <thead>
                    <tr>
                        <th>{"Date"}</th>
                        <th>{"Description"}</th>
                        <th>{"Category"}</th>
                        <th>{"Account"}</th>
                        <th>{"Amount"}</th>
                        <th>{"Actions"}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>{"2023-10-01"}</td>
                        <td>{"Grocery Store"}</td>
                        <td>{"Food"}</td>
                        <td>{"Main Checking"}</td>
                        <td class="text-error">{"-$150.00"}</td>
                        <td>
                            <button class="btn btn-ghost btn-xs">{"Edit"}</button>
                            <button class="btn btn-ghost btn-xs text-error">{"Delete"}</button>
                        </td>
                    </tr>
                     <tr>
                        <td>{"2023-10-02"}</td>
                        <td>{"Salary"}</td>
                        <td>{"Income"}</td>
                        <td>{"Main Checking"}</td>
                        <td class="text-success">{"$3,000.00"}</td>
                        <td>
                            <button class="btn btn-ghost btn-xs">{"Edit"}</button>
                            <button class="btn btn-ghost btn-xs text-error">{"Delete"}</button>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    }
}
