use yew::prelude::*;

#[function_component(RecurringList)]
pub fn recurring_list() -> Html {
    html! {
        <div class="overflow-x-auto bg-base-100 shadow rounded-box">
            <table class="table">
                <thead>
                    <tr>
                        <th>{"Name"}</th>
                        <th>{"Amount"}</th>
                        <th>{"Interval"}</th>
                        <th>{"Next Due"}</th>
                        <th>{"Actions"}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>{"Netflix"}</td>
                        <td class="text-error">{"-$15.00"}</td>
                        <td>{"Monthly"}</td>
                        <td>{"2023-10-15"}</td>
                        <td>
                            <button class="btn btn-xs btn-success">{"Pay"}</button>
                            <button class="btn btn-xs btn-ghost">{"Edit"}</button>
                        </td>
                    </tr>
                    <tr>
                        <td>{"Gym Membership"}</td>
                        <td class="text-error">{"-$45.00"}</td>
                        <td>{"Monthly"}</td>
                        <td>{"2023-10-20"}</td>
                        <td>
                            <button class="btn btn-xs btn-success">{"Pay"}</button>
                            <button class="btn btn-xs btn-ghost">{"Edit"}</button>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    }
}
