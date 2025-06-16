use chrono::NaiveDate;
use model::entities::manual_account_state;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

/// Gets the latest manual account state before the given date.
pub async fn get_latest_manual_state(
    db: &DatabaseConnection,
    account_id: i32,
    before_date: NaiveDate,
) -> Result<Option<manual_account_state::Model>, Box<dyn std::error::Error>> {
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
    
    Ok(state)
}

/// Gets all manual account states within the given date range.
pub async fn get_manual_states_in_range(
    db: &DatabaseConnection,
    account_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<manual_account_state::Model>, Box<dyn std::error::Error>> {
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
    
    Ok(states)
}