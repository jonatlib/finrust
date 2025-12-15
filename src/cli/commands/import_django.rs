use anyhow::{Context, Result};
use chrono::{Months, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, EntityTrait, Set};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use tracing::{debug, info, trace, warn};

use model::transaction::TransactionGenerator;

use model::entities::{
    account, category, manual_account_state, one_off_transaction, one_off_transaction_tag,
    recurring_transaction, recurring_transaction_instance, recurring_transaction_tag, tag, user,
};

/// Main structure for Django dump
#[derive(Debug, Deserialize)]
struct DjangoRecord {
    model: String,
    #[serde(deserialize_with = "deserialize_pk")]
    pk: i32,
    fields: serde_json::Value,
}

/// Custom deserializer for pk field that handles both string and integer PKs
fn deserialize_pk<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let value: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;

    match value {
        serde_json::Value::Number(n) => {
            n.as_i64()
                .and_then(|v| i32::try_from(v).ok())
                .ok_or_else(|| D::Error::custom("pk number out of i32 range"))
        }
        serde_json::Value::String(_s) => {
            // For string PKs (like session keys), we'll just skip them by returning 0
            // The importer will ignore these records anyway
            Ok(0)
        }
        _ => Err(D::Error::custom("pk must be a number or string")),
    }
}

/// Django Currency Model
#[derive(Debug, Deserialize)]
struct DjangoCurrency {
    name: String,
    prefix: Option<String>,
    suffix: Option<String>,
}

/// Django Account Model
#[derive(Debug, Deserialize)]
struct DjangoAccount {
    name: String,
    description: String,
    currency: i32,
    show_in_overview: bool,
    include_in_statistics: bool,
    owner: i32,
    ledger_name: Option<i32>,
    tags: Vec<i32>,
    allowed_users: Vec<i32>,
}

/// Django Category Model
#[derive(Debug, Deserialize)]
struct DjangoCategory {
    name: String,
    color: String,
    parent: Option<i32>,
    ledger_name: Option<i32>,
    lft: i32,
    rght: i32,
    tree_id: i32,
    level: i32,
}

/// Django Regular Transaction Model
#[derive(Debug, Deserialize)]
struct DjangoRegularTransaction {
    name: String,
    description: String,
    amount: String,
    include_in_statistics: bool,
    category: Option<i32>,
    target_account: i32,
    counterparty_account: Option<i32>,
    ledger_name: Option<i32>,
    period: String,
    billing_start: String,
    billing_end: Option<String>,
    tag: Vec<i32>,
}

/// Django Extra Transaction Model
#[derive(Debug, Deserialize)]
struct DjangoExtraTransaction {
    name: String,
    description: String,
    amount: String,
    include_in_statistics: bool,
    category: Option<i32>,
    target_account: i32,
    counterparty_account: Option<i32>,
    ledger_name: Option<i32>,
    date: String,
    tag: Vec<i32>,
}

/// Django Manual Account State Model
#[derive(Debug, Deserialize)]
struct DjangoManualAccountState {
    account: i32,
    date: String,
    amount: String,
}

/// Django Tag Model
#[derive(Debug, Deserialize)]
struct DjangoTag {
    name: String,
    #[serde(default)]
    description: Option<String>,
}

pub async fn import_django(json_path: &str, database_url: &str) -> Result<()> {
    trace!("Entering import_django function");
    info!("Starting Django data import");
    debug!("JSON path: {}", json_path);
    debug!("Database URL: {}", database_url);

    // Connect to database
    trace!("Connecting to database");
    let db = Database::connect(database_url)
        .await
        .context("Failed to connect to database")?;
    info!("Connected to database");

    // Load JSON file
    trace!("Loading JSON file");
    let path = Path::new(json_path);
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", json_path))?;

    info!("Parsing JSON data...");
    let records: Vec<DjangoRecord> = serde_json::from_reader(file)
        .context("Failed to parse JSON")?;
    info!("Loaded {} records from Django dump", records.len());

    // Create default user if needed
    let default_user = ensure_default_user(&db).await?;
    info!("Default user ID: {}", default_user.id);

    // Process records by type
    let mut currency_map = HashMap::new();
    let mut account_map = HashMap::new();
    let mut category_map = HashMap::new();
    let mut tag_map = HashMap::new();

    // First pass: Import currencies (to get the default currency code)
    info!("Importing currencies...");
    let mut default_currency_code = "CZK".to_string();
    for record in records.iter() {
        if record.model == "account.currencymodel" {
            let currency: DjangoCurrency = serde_json::from_value(record.fields.clone())?;
            currency_map.insert(record.pk, currency.name.clone());
            default_currency_code = currency.name.clone();
            debug!("Mapped currency {} -> {}", record.pk, currency.name);
        }
    }
    info!("Imported {} currencies", currency_map.len());

    // Second pass: Import tags
    info!("Importing tags...");
    for record in records.iter() {
        if record.model == "account.tagmodel" {
            let django_tag: DjangoTag = serde_json::from_value(record.fields.clone())?;
            let new_tag = tag::ActiveModel {
                name: Set(django_tag.name.clone()),
                description: Set(django_tag.description.clone()),
                parent_id: Set(None),
                ledger_name: Set(None),
                ..Default::default()
            };

            let inserted_tag = new_tag.insert(&db).await?;
            tag_map.insert(record.pk, inserted_tag.id);
            debug!("Imported tag {} -> ID {}", django_tag.name, inserted_tag.id);
        }
    }
    info!("Imported {} tags", tag_map.len());

    // Third pass: Import categories
    info!("Importing categories...");
    let mut categories_to_process: Vec<(i32, DjangoCategory)> = Vec::new();
    for record in records.iter() {
        if record.model == "account.categorymodel" {
            let category: DjangoCategory = serde_json::from_value(record.fields.clone())?;
            categories_to_process.push((record.pk, category));
        }
    }

    // Sort by level to process parents before children
    categories_to_process.sort_by_key(|(_, cat)| cat.level);

    for (pk, django_category) in categories_to_process {
        let parent_id = django_category.parent.and_then(|p| category_map.get(&p).copied());

        let new_category = category::ActiveModel {
            name: Set(django_category.name.clone()),
            description: Set(Some(format!("Color: {}", django_category.color))),
            parent_id: Set(parent_id),
            ..Default::default()
        };

        let inserted_category = new_category.insert(&db).await?;
        category_map.insert(pk, inserted_category.id);
        debug!("Imported category {} -> ID {} (parent: {:?})",
               django_category.name, inserted_category.id, parent_id);
    }
    info!("Imported {} categories", category_map.len());

    // Fourth pass: Import accounts
    info!("Importing accounts...");
    for record in records.iter() {
        if record.model == "account.moneyaccountmodel" {
            let django_account: DjangoAccount = serde_json::from_value(record.fields.clone())?;

            let currency_code = currency_map.get(&django_account.currency)
                .unwrap_or(&default_currency_code)
                .clone();

            let new_account = account::ActiveModel {
                name: Set(django_account.name.clone()),
                description: Set(Some(django_account.description.clone())),
                currency_code: Set(currency_code),
                owner_id: Set(default_user.id),
                include_in_statistics: Set(django_account.include_in_statistics),
                ledger_name: Set(None),
                account_kind: Set(account::AccountKind::RealAccount),
                ..Default::default()
            };

            let inserted_account = new_account.insert(&db).await?;
            account_map.insert(record.pk, inserted_account.id);
            debug!("Imported account {} -> ID {}", django_account.name, inserted_account.id);

            // Import account tags
            for tag_pk in &django_account.tags {
                if let Some(&tag_id) = tag_map.get(tag_pk) {
                    let account_tag = model::entities::account_tag::ActiveModel {
                        account_id: Set(inserted_account.id),
                        tag_id: Set(tag_id),
                        ..Default::default()
                    };
                    account_tag.insert(&db).await?;
                    debug!("Linked account {} to tag {}", inserted_account.id, tag_id);
                }
            }
        }
    }
    info!("Imported {} accounts", account_map.len());

    // Fifth pass: Import manual account states
    info!("Importing manual account states...");
    let mut state_count = 0;
    for record in records.iter() {
        if record.model == "account.manualaccountstatemodel" {
            let state: DjangoManualAccountState = serde_json::from_value(record.fields.clone())?;

            if let Some(&account_id) = account_map.get(&state.account) {
                let amount = state.amount.parse::<Decimal>()
                    .unwrap_or_else(|_| Decimal::ZERO);
                let date = NaiveDate::parse_from_str(&state.date, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Utc::now().naive_utc().date());

                let new_state = manual_account_state::ActiveModel {
                    account_id: Set(account_id),
                    date: Set(date),
                    amount: Set(amount),
                    ..Default::default()
                };

                new_state.insert(&db).await?;
                state_count += 1;
                debug!("Imported account state for account {} on {}: {}",
                       account_id, date, amount);
            }
        }
    }
    info!("Imported {} manual account states", state_count);

    // Sixth pass: Import recurring transactions
    info!("Importing recurring transactions...");
    let mut recurring_count = 0;
    let mut imported_recurring_transactions = Vec::new();
    for record in records.iter() {
        if record.model == "account.regulartransactionmodel" {
            let tx: DjangoRegularTransaction = serde_json::from_value(record.fields.clone())?;

            if let Some(&target_account_id) = account_map.get(&tx.target_account) {
                let amount = tx.amount.parse::<Decimal>().unwrap_or_else(|_| Decimal::ZERO);
                let start_date = NaiveDate::parse_from_str(&tx.billing_start, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Utc::now().naive_utc().date());
                let end_date = tx.billing_end.as_ref()
                    .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

                let period = match tx.period.as_str() {
                    "Daily" => recurring_transaction::RecurrencePeriod::Daily,
                    "Weekly" => recurring_transaction::RecurrencePeriod::Weekly,
                    "Work-Day" | "WorkDay" => recurring_transaction::RecurrencePeriod::WorkDay,
                    "Monthly" => recurring_transaction::RecurrencePeriod::Monthly,
                    "Quarterly" => recurring_transaction::RecurrencePeriod::Quarterly,
                    "Half-Yearly" | "HalfYearly" => recurring_transaction::RecurrencePeriod::HalfYearly,
                    "Yearly" => recurring_transaction::RecurrencePeriod::Yearly,
                    _ => {
                        warn!("Unknown period '{}', defaulting to Monthly", tx.period);
                        recurring_transaction::RecurrencePeriod::Monthly
                    }
                };

                let source_account_id = tx.counterparty_account
                    .and_then(|id| account_map.get(&id).copied());
                let category_id = tx.category
                    .and_then(|id| category_map.get(&id).copied());

                let new_tx = recurring_transaction::ActiveModel {
                    name: Set(tx.name.clone()),
                    description: Set(Some(tx.description.clone())),
                    amount: Set(amount),
                    start_date: Set(start_date),
                    end_date: Set(end_date),
                    period: Set(period),
                    include_in_statistics: Set(tx.include_in_statistics),
                    target_account_id: Set(target_account_id),
                    source_account_id: Set(source_account_id),
                    category_id: Set(category_id),
                    ledger_name: Set(None),
                    ..Default::default()
                };

                let inserted_tx = new_tx.insert(&db).await?;
                recurring_count += 1;
                debug!("Imported recurring transaction {} -> ID {}", tx.name, inserted_tx.id);

                // Store the inserted transaction for instance generation
                imported_recurring_transactions.push(inserted_tx.clone());

                // Import transaction tags
                for tag_pk in &tx.tag {
                    if let Some(&tag_id) = tag_map.get(tag_pk) {
                        let tx_tag = recurring_transaction_tag::ActiveModel {
                            transaction_id: Set(inserted_tx.id),
                            tag_id: Set(tag_id),
                            ..Default::default()
                        };
                        tx_tag.insert(&db).await?;
                        debug!("Linked recurring transaction {} to tag {}", inserted_tx.id, tag_id);
                    }
                }
            }
        }
    }
    info!("Imported {} recurring transactions", recurring_count);

    // Generate instances for recurring transactions (last 20 months)
    info!("Generating instances for recurring transactions...");
    let today = chrono::Utc::now().naive_utc().date();
    let start_date = today.checked_sub_months(Months::new(20))
        .unwrap_or(today);

    let mut instance_count = 0;
    for recurring_tx in imported_recurring_transactions {
        // Use the transaction generator to get all dates
        let transactions = recurring_tx.generate_transactions(
            start_date,
            today,
            today,
            &db,
        ).await;

        // Create an instance for each generated transaction
        for tx in transactions {
            let new_instance = recurring_transaction_instance::ActiveModel {
                recurring_transaction_id: Set(recurring_tx.id),
                status: Set(recurring_transaction_instance::InstanceStatus::Paid),
                due_date: Set(tx.date()),
                expected_amount: Set(recurring_tx.amount),
                paid_date: Set(Some(tx.date())),
                paid_amount: Set(Some(recurring_tx.amount)),
                reconciled_imported_transaction_id: Set(None),
                category_id: Set(recurring_tx.category_id),
                ..Default::default()
            };

            new_instance.insert(&db).await?;
            instance_count += 1;
            debug!("Created paid instance for recurring transaction {} on {}",
                   recurring_tx.id, tx.date());
        }
    }
    info!("Generated {} instances for recurring transactions", instance_count);

    // Seventh pass: Import one-off transactions
    info!("Importing one-off transactions...");
    let mut oneoff_count = 0;
    for record in records.iter() {
        if record.model == "account.extratransactionmodel" {
            let tx: DjangoExtraTransaction = serde_json::from_value(record.fields.clone())?;

            if let Some(&target_account_id) = account_map.get(&tx.target_account) {
                let amount = tx.amount.parse::<Decimal>().unwrap_or_else(|_| Decimal::ZERO);
                let date = NaiveDate::parse_from_str(&tx.date, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Utc::now().naive_utc().date());

                let source_account_id = tx.counterparty_account
                    .and_then(|id| account_map.get(&id).copied());
                let category_id = tx.category
                    .and_then(|id| category_map.get(&id).copied());

                let new_tx = one_off_transaction::ActiveModel {
                    name: Set(tx.name.clone()),
                    description: Set(Some(tx.description.clone())),
                    amount: Set(amount),
                    date: Set(date),
                    include_in_statistics: Set(tx.include_in_statistics),
                    target_account_id: Set(target_account_id),
                    source_account_id: Set(source_account_id),
                    category_id: Set(category_id),
                    ledger_name: Set(None),
                    linked_import_id: Set(None),
                    ..Default::default()
                };

                let inserted_tx = new_tx.insert(&db).await?;
                oneoff_count += 1;
                debug!("Imported one-off transaction {} -> ID {} on {}",
                       tx.name, inserted_tx.id, date);

                // Import transaction tags
                for tag_pk in &tx.tag {
                    if let Some(&tag_id) = tag_map.get(tag_pk) {
                        let tx_tag = one_off_transaction_tag::ActiveModel {
                            transaction_id: Set(inserted_tx.id),
                            tag_id: Set(tag_id),
                            ..Default::default()
                        };
                        tx_tag.insert(&db).await?;
                        debug!("Linked one-off transaction {} to tag {}", inserted_tx.id, tag_id);
                    }
                }
            }
        }
    }
    info!("Imported {} one-off transactions", oneoff_count);

    info!("Django data import completed successfully!");
    info!("Summary:");
    info!("  - Tags: {}", tag_map.len());
    info!("  - Categories: {}", category_map.len());
    info!("  - Accounts: {}", account_map.len());
    info!("  - Manual Account States: {}", state_count);
    info!("  - Recurring Transactions: {}", recurring_count);
    info!("  - Recurring Transaction Instances: {}", instance_count);
    info!("  - One-off Transactions: {}", oneoff_count);

    Ok(())
}

async fn ensure_default_user(db: &DatabaseConnection) -> Result<user::Model> {
    use model::entities::prelude::User;

    // Try to find existing user
    match User::find().one(db).await? {
        Some(user) => {
            info!("Using existing user: {}", user.username);
            Ok(user)
        }
        None => {
            // Create default user
            info!("Creating default user");
            let new_user = user::ActiveModel {
                username: Set("default".to_string()),
                ..Default::default()
            };
            let user = new_user.insert(db).await?;
            info!("Created default user with ID: {}", user.id);
            Ok(user)
        }
    }
}
