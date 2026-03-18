use anyhow::{Context, Result};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, Database, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

use model::entities::account;

/// PascalCase account kind for human-readable YAML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum AccountKind {
    RealAccount,
    Savings,
    Investment,
    Debt,
    Other,
    Goal,
    Allowance,
    Shared,
    EmergencyFund,
    Equity,
    House,
    Tax,
}

impl From<account::AccountKind> for AccountKind {
    fn from(kind: account::AccountKind) -> Self {
        match kind {
            account::AccountKind::RealAccount => AccountKind::RealAccount,
            account::AccountKind::Savings => AccountKind::Savings,
            account::AccountKind::Investment => AccountKind::Investment,
            account::AccountKind::Debt => AccountKind::Debt,
            account::AccountKind::Other => AccountKind::Other,
            account::AccountKind::Goal => AccountKind::Goal,
            account::AccountKind::Allowance => AccountKind::Allowance,
            account::AccountKind::Shared => AccountKind::Shared,
            account::AccountKind::EmergencyFund => AccountKind::EmergencyFund,
            account::AccountKind::Equity => AccountKind::Equity,
            account::AccountKind::House => AccountKind::House,
            account::AccountKind::Tax => AccountKind::Tax,
        }
    }
}

impl From<AccountKind> for account::AccountKind {
    fn from(kind: AccountKind) -> Self {
        match kind {
            AccountKind::RealAccount => account::AccountKind::RealAccount,
            AccountKind::Savings => account::AccountKind::Savings,
            AccountKind::Investment => account::AccountKind::Investment,
            AccountKind::Debt => account::AccountKind::Debt,
            AccountKind::Other => account::AccountKind::Other,
            AccountKind::Goal => account::AccountKind::Goal,
            AccountKind::Allowance => account::AccountKind::Allowance,
            AccountKind::Shared => account::AccountKind::Shared,
            AccountKind::EmergencyFund => account::AccountKind::EmergencyFund,
            AccountKind::Equity => account::AccountKind::Equity,
            AccountKind::House => account::AccountKind::House,
            AccountKind::Tax => account::AccountKind::Tax,
        }
    }
}

/// Per-account customization entry. All fields except `name` are optional;
/// only present fields will be applied as overrides during import.
#[derive(Debug, Deserialize, Serialize)]
pub struct AccountOverlayEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_kind: Option<AccountKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_amount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_liquid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_in_statistics: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountOverlayFile {
    pub accounts: Vec<AccountOverlayEntry>,
}

/// Export current account customizations from the database to a YAML file.
pub async fn export_account_overlay(database_url: &str, output_path: &str) -> Result<()> {
    let db = Database::connect(database_url)
        .await
        .context("Failed to connect to database")?;

    let accounts = account::Entity::find()
        .all(&db)
        .await
        .context("Failed to load accounts")?;

    let entries: Vec<AccountOverlayEntry> = accounts
        .into_iter()
        .map(|a| AccountOverlayEntry {
            name: a.name,
            color: a.color,
            account_kind: Some(a.account_kind.into()),
            target_amount: a.target_amount,
            is_liquid: Some(a.is_liquid),
            include_in_statistics: Some(a.include_in_statistics),
        })
        .collect();

    let overlay = AccountOverlayFile { accounts: entries };
    let yaml = serde_yaml::to_string(&overlay).context("Failed to serialize overlay to YAML")?;

    fs::write(output_path, &yaml)
        .with_context(|| format!("Failed to write overlay file: {}", output_path))?;

    info!("Exported {} account(s) to {}", overlay.accounts.len(), output_path);
    Ok(())
}

/// Read an overlay YAML file and apply matching entries to accounts in the database.
pub async fn apply_account_overlay(database_url: &str, overlay_path: &str) -> Result<()> {
    let db = Database::connect(database_url)
        .await
        .context("Failed to connect to database")?;

    let path = Path::new(overlay_path);
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read overlay file: {}", overlay_path))?;

    let overlay: AccountOverlayFile =
        serde_yaml::from_str(&contents).context("Failed to parse overlay YAML")?;

    let all_accounts = account::Entity::find()
        .all(&db)
        .await
        .context("Failed to load accounts")?;

    let account_by_name: HashMap<String, account::Model> = all_accounts
        .into_iter()
        .map(|a| (a.name.clone(), a))
        .collect();

    let mut applied = 0;
    for entry in &overlay.accounts {
        let Some(existing) = account_by_name.get(&entry.name) else {
            warn!("Overlay: no account named '{}' found, skipping", entry.name);
            continue;
        };

        let mut model = account::ActiveModel {
            id: Set(existing.id),
            ..Default::default()
        };

        let mut changed = false;

        if let Some(ref color) = entry.color {
            model.color = Set(Some(color.clone()));
            changed = true;
        }
        if let Some(kind) = entry.account_kind {
            model.account_kind = Set(kind.into());
            changed = true;
        }
        if let Some(target) = entry.target_amount {
            model.target_amount = Set(Some(target));
            changed = true;
        }
        if let Some(liquid) = entry.is_liquid {
            model.is_liquid = Set(liquid);
            changed = true;
        }
        if let Some(stats) = entry.include_in_statistics {
            model.include_in_statistics = Set(stats);
            changed = true;
        }

        if changed {
            model.update(&db).await.with_context(|| {
                format!("Failed to update account '{}'", entry.name)
            })?;
            applied += 1;
            info!("Overlay applied to account '{}'", entry.name);
        }
    }

    info!(
        "Overlay complete: {}/{} entries applied ({} unmatched)",
        applied,
        overlay.accounts.len(),
        overlay.accounts.len() - applied,
    );
    Ok(())
}
