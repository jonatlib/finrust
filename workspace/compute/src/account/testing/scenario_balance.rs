use async_trait::async_trait;
use sea_orm::DbErr;

use super::setup_db;
use crate::account::testing::{TestScenario, TestScenarioBuilder};

pub struct ScenarioBalance {}

#[async_trait]
impl TestScenarioBuilder for ScenarioBalance {
    async fn get_scenario(&self) -> Result<TestScenario, DbErr> {
        let db = setup_db().await?;
        todo!()
    }
}
