use chrono::NaiveDate;
use model::entities::manual_account_state;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use tracing::{debug, instrument, trace};

use crate::error::Result;

/// Gets the latest manual account state before the given date.
#[instrument(skip(db), fields(account_id = account_id, before_date = %before_date))]
pub async fn get_latest_manual_state(
    db: &DatabaseConnection,
    account_id: i32,
    before_date: NaiveDate,
) -> Result<Option<manual_account_state::Model>> {
    trace!("Getting latest manual account state for account_id={} before {}", account_id, before_date);

    let state = manual_account_state::Entity::find()
        .filter(
            Condition::all()
                .add(manual_account_state::Column::AccountId.eq(account_id))
                .add(manual_account_state::Column::Date.lte(before_date)),
        )
        .order_by_desc(manual_account_state::Column::Date)
        .limit(1)
        .one(db)
        .await?;

    match &state {
        Some(s) => debug!("Found manual state for account_id={}: date={}, amount={}", account_id, s.date, s.amount),
        None => debug!("No manual state found for account_id={} before {}", account_id, before_date),
    }

    Ok(state)
}

/// Gets all manual account states within the given date range.
#[instrument(skip(db), fields(account_id = account_id, start_date = %start_date, end_date = %end_date))]
pub async fn get_manual_states_in_range(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<manual_account_state::Model>> {
    trace!("Getting manual account states for account_id={} from {} to {}", account_id, start_date, end_date);

    let states = manual_account_state::Entity::find()
        .filter(
            Condition::all()
                .add(manual_account_state::Column::AccountId.eq(account_id))
                .add(manual_account_state::Column::Date.gte(start_date))
                .add(manual_account_state::Column::Date.lte(end_date)),
        )
        .order_by_asc(manual_account_state::Column::Date)
        .all(db)
        .await?;

    debug!("Found {} manual states for account_id={} in date range {} to {}", 
           states.len(), account_id, start_date, end_date);

    for state in &states {
        trace!("Manual state: account_id={}, date={}, amount={}", account_id, state.date, state.amount);
    }

    Ok(states)
}
