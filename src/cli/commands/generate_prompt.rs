use anyhow::Result;
use chrono::{Datelike, Months, NaiveDate, Utc};
use compute::account::utils::generate_occurrences;
use compute::{account::AccountStateCalculator, default_compute};
use model::entities::{
    account::{self, AccountKind},
    category, one_off_transaction, recurring_income, recurring_transaction,
    recurring_transaction_instance,
};
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as FmtWrite;
use std::str::FromStr;
use tracing::info;

pub async fn generate_prompt(database_url: &str, months_back: u32) -> Result<()> {
    info!("Generating financial assessment prompt");
    let db: DatabaseConnection = Database::connect(database_url).await?;
    let prompt = build_prompt(&db, months_back).await?;
    print!("{}", prompt);
    Ok(())
}

/// Build the full financial assessment prompt from DB data.
/// Shared between the CLI command and the API handler.
pub async fn build_prompt(db: &DatabaseConnection, months_back: u32) -> Result<String> {
    let today = Utc::now().date_naive();

    let accounts: Vec<account::Model> = account::Entity::find().all(db).await?;
    let categories: Vec<category::Model> = category::Entity::find().all(db).await?;

    if accounts.is_empty() {
        return Ok("No accounts found in the database.".to_string());
    }

    let compute = default_compute(Some(today));
    let mut prompt = String::with_capacity(16_000);

    write_system_prompt(&mut prompt);
    write_account_overview(&mut prompt, &accounts);
    write_monthly_balances(&mut prompt, &compute, db, &accounts, today, months_back).await;
    write_income_summary(&mut prompt, db, &accounts, today, months_back).await;
    write_category_breakdown(&mut prompt, db, &accounts, &categories, today, months_back).await;
    write_user_prompt(&mut prompt);

    Ok(prompt)
}

fn write_system_prompt(out: &mut String) {
    out.push_str(r#"<system>
You are the world's best financial advisor with decades of experience in personal finance,
wealth management, and financial planning. You are given structured financial data about a
person's accounts, monthly balances, income, and spending by category.

DATA FORMAT NOTES:
- All monetary amounts are in the account's currency (see currency_code per account)
- Negative balances on Debt accounts represent outstanding debt
- Monthly balances are end-of-month snapshots
- Category spending amounts are negative (outflows); income amounts are positive (inflows)
- "Target amount" on accounts represents a savings/funding goal
- Account kinds: RealAccount (checking/operating), Savings, Investment, Debt, Goal,
  EmergencyFund, Allowance (personal spending), Shared (joint), Equity, House, Tax, Other
- is_liquid indicates whether the account can be quickly converted to cash

YOUR TASK:
1. Assess the current financial situation holistically
2. Analyze the trajectory — are things improving or deteriorating?
3. Check if money is scattered across too many accounts without clear purpose
4. Review goals (target amounts) and whether they are realistic given current flows
5. Suggest concrete, measurable goals for 12, 24, and 36 months
6. Provide actionable recommendations prioritized by impact
7. Flag any risks or concerns you see
</system>

"#);
}

fn account_kind_str(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::RealAccount => "RealAccount (Checking/Operating)",
        AccountKind::Savings => "Savings",
        AccountKind::Investment => "Investment",
        AccountKind::Debt => "Debt",
        AccountKind::Other => "Other",
        AccountKind::Goal => "Goal",
        AccountKind::Allowance => "Allowance (Personal Spending)",
        AccountKind::Shared => "Shared (Joint)",
        AccountKind::EmergencyFund => "Emergency Fund",
        AccountKind::Equity => "Equity",
        AccountKind::House => "House",
        AccountKind::Tax => "Tax",
    }
}

fn write_account_overview(out: &mut String, accounts: &[account::Model]) {
    out.push_str("## ACCOUNTS OVERVIEW\n\n");
    out.push_str("| Name | Type | Currency | Target | Liquid | Description |\n");
    out.push_str("|------|------|----------|--------|--------|-------------|\n");
    for a in accounts {
        let target = a
            .target_amount
            .map(|t| t.round_dp(0).to_string())
            .unwrap_or_else(|| "-".into());
        let desc = a.description.as_deref().unwrap_or("-");
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            a.name,
            account_kind_str(a.account_kind),
            a.currency_code,
            target,
            if a.is_liquid { "Yes" } else { "No" },
            desc,
        );
    }
    out.push('\n');
}

async fn write_monthly_balances(
    out: &mut String,
    compute: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    today: NaiveDate,
    months_back: u32,
) {
    out.push_str("## MONTHLY BALANCE SHEET (end-of-month balances)\n\n");

    let range_start = today
        .checked_sub_months(Months::new(months_back))
        .unwrap_or(today)
        .with_day(1)
        .unwrap_or(today);
    let range_end = get_last_day_of_month(today.year(), today.month());

    let df = match compute
        .compute_account_state(db, accounts, range_start, range_end)
        .await
    {
        Ok(df) => df,
        Err(e) => {
            let _ = writeln!(out, "Error computing balances: {}\n", e);
            return;
        }
    };

    let balances = match extract_end_of_month_balances(&df, range_start, today, accounts) {
        Ok(b) => b,
        Err(e) => {
            let _ = writeln!(out, "Error extracting balances: {}\n", e);
            return;
        }
    };

    // Collect month labels
    let mut month_labels: Vec<String> = Vec::new();
    let mut cursor = range_start;
    while cursor <= today {
        month_labels.push(format!("{}-{:02}", cursor.year(), cursor.month()));
        cursor = cursor
            .checked_add_months(Months::new(1))
            .unwrap_or(today);
        if cursor > today && month_labels.last().map(|l| l.as_str())
            != Some(&format!("{}-{:02}", today.year(), today.month()))
        {
            month_labels.push(format!("{}-{:02}", today.year(), today.month()));
            break;
        }
    }
    month_labels.sort();
    month_labels.dedup();

    // Header
    out.push_str("| Account |");
    for label in &month_labels {
        let _ = write!(out, " {} |", label);
    }
    out.push('\n');

    out.push_str("|---------|");
    for _ in &month_labels {
        out.push_str("--------|");
    }
    out.push('\n');

    // Per account row
    for a in accounts {
        let _ = write!(out, "| {} |", a.name);
        for label in &month_labels {
            let val = balances
                .get(&a.id)
                .and_then(|m| m.get(label.as_str()))
                .map(|d| d.round_dp(0).to_string())
                .unwrap_or_else(|| "-".into());
            let _ = write!(out, " {} |", val);
        }
        out.push('\n');
    }

    // Total row
    out.push_str("| **TOTAL** |");
    for label in &month_labels {
        let total: Decimal = accounts
            .iter()
            .filter_map(|a| {
                balances
                    .get(&a.id)
                    .and_then(|m| m.get(label.as_str()))
            })
            .copied()
            .sum();
        let _ = write!(out, " {} |", total.round_dp(0));
    }
    out.push_str("\n\n");
}

/// Extract end-of-month balances per account from the batch DataFrame.
fn extract_end_of_month_balances(
    df: &DataFrame,
    _range_start: NaiveDate,
    today: NaiveDate,
    accounts: &[account::Model],
) -> std::result::Result<HashMap<i32, HashMap<String, Decimal>>, String> {
    let account_col = df
        .column("account_id")
        .or_else(|_| df.column("account"))
        .map_err(|e| format!("Missing account column: {e}"))?;
    let date_col = df
        .column("date")
        .map_err(|e| format!("Missing date column: {e}"))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| format!("Missing balance column: {e}"))?;

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

    // Collect all data points grouped by (account_id, year-month)
    // For each month, we want the balance at the latest date within that month.
    let mut month_data: HashMap<i32, HashMap<String, (i64, Decimal)>> = HashMap::new();

    for i in 0..df.height() {
        let aid = account_col
            .get(i)
            .map_err(|e| format!("row {i}: {e}"))?
            .try_extract::<i32>()
            .map_err(|e| format!("row {i}: {e}"))?;
        let date_epoch = date_col
            .get(i)
            .map_err(|e| format!("row {i}: {e}"))?
            .try_extract::<i64>()
            .map_err(|e| format!("row {i}: {e}"))?;
        let bal_any = balance_col
            .get(i)
            .map_err(|e| format!("row {i}: {e}"))?;
        let bal_str = match bal_any {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        };
        let bal = Decimal::from_str(&bal_str).map_err(|e| format!("'{bal_str}': {e}"))?;

        let date = epoch
            .checked_add_signed(chrono::Duration::days(date_epoch))
            .ok_or_else(|| format!("date overflow at row {i}"))?;

        // Only include dates up to today
        if date > today {
            continue;
        }

        let month_key = format!("{}-{:02}", date.year(), date.month());
        let entry = month_data.entry(aid).or_default();
        match entry.get(&month_key) {
            Some((existing_date, _)) if date_epoch > *existing_date => {
                entry.insert(month_key, (date_epoch, bal));
            }
            None => {
                entry.insert(month_key, (date_epoch, bal));
            }
            _ => {}
        }
    }

    // For accounts with no data, try to provide empty maps
    let mut result: HashMap<i32, HashMap<String, Decimal>> = HashMap::new();
    for a in accounts {
        let account_months = month_data.remove(&a.id).unwrap_or_default();
        let simplified: HashMap<String, Decimal> = account_months
            .into_iter()
            .map(|(k, (_, bal))| (k, bal))
            .collect();
        result.insert(a.id, simplified);
    }
    Ok(result)
}

async fn write_income_summary(
    out: &mut String,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    today: NaiveDate,
    months_back: u32,
) {
    out.push_str("## INCOME SUMMARY (per year)\n\n");

    let range_start = today
        .checked_sub_months(Months::new(months_back))
        .unwrap_or(today)
        .with_day(1)
        .unwrap_or(today);

    let incomes: Vec<recurring_income::Model> = match recurring_income::Entity::find()
        .filter(recurring_income::Column::IsSimulated.eq(false))
        .all(db)
        .await
    {
        Ok(i) => i,
        Err(e) => {
            let _ = writeln!(out, "Error loading incomes: {}\n", e);
            return;
        }
    };

    if incomes.is_empty() {
        out.push_str("No recurring income definitions found.\n\n");
        return;
    }

    let account_map: HashMap<i32, &account::Model> =
        accounts.iter().map(|a| (a.id, a)).collect();

    // yearly_totals: year -> total income
    let mut yearly_totals: BTreeMap<i32, Decimal> = BTreeMap::new();
    // per_income: (income_name, target_account_name, amount, period) -> yearly totals
    let mut per_income: Vec<(String, String, BTreeMap<i32, Decimal>)> = Vec::new();

    for income in &incomes {
        let occurrences = generate_occurrences(
            income.start_date,
            income.end_date,
            &income.period,
            range_start,
            today,
        );

        let acct_name = account_map
            .get(&income.target_account_id)
            .map(|a| a.name.as_str())
            .unwrap_or("Unknown");

        let mut income_yearly: BTreeMap<i32, Decimal> = BTreeMap::new();
        for date in &occurrences {
            let year = date.year();
            *income_yearly.entry(year).or_insert(Decimal::ZERO) += income.amount;
            *yearly_totals.entry(year).or_insert(Decimal::ZERO) += income.amount;
        }

        let label = income
            .name
            .clone();
        per_income.push((label, acct_name.to_string(), income_yearly));
    }

    // Print yearly totals
    let years: Vec<i32> = yearly_totals.keys().copied().collect();
    out.push_str("| Income Source | Target Account |");
    for y in &years {
        let _ = write!(out, " {} |", y);
    }
    out.push('\n');

    out.push_str("|--------------|----------------|");
    for _ in &years {
        out.push_str("--------|");
    }
    out.push('\n');

    for (name, acct, yearly) in &per_income {
        let _ = write!(out, "| {} | {} |", name, acct);
        for y in &years {
            let val = yearly
                .get(y)
                .map(|d| d.round_dp(0).to_string())
                .unwrap_or_else(|| "-".into());
            let _ = write!(out, " {} |", val);
        }
        out.push('\n');
    }

    out.push_str("| **TOTAL** | |");
    for y in &years {
        let total = yearly_totals.get(y).copied().unwrap_or(Decimal::ZERO);
        let _ = write!(out, " {} |", total.round_dp(0));
    }
    out.push_str("\n\n");
}

async fn write_category_breakdown(
    out: &mut String,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    categories: &[category::Model],
    today: NaiveDate,
    months_back: u32,
) {
    out.push_str("## SPENDING & INCOME BY CATEGORY (average per year)\n\n");

    let range_start = today
        .checked_sub_months(Months::new(months_back))
        .unwrap_or(today)
        .with_day(1)
        .unwrap_or(today);

    let account_ids: Vec<i32> = accounts.iter().map(|a| a.id).collect();

    // Load one-off transactions
    let one_off_txns = match one_off_transaction::Entity::find()
        .filter(one_off_transaction::Column::Date.between(range_start, today))
        .filter(one_off_transaction::Column::CategoryId.is_not_null())
        .filter(one_off_transaction::Column::TargetAccountId.is_in(account_ids.clone()))
        .all(db)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            let _ = writeln!(out, "Error loading transactions: {}\n", e);
            return;
        }
    };

    // Load recurring transactions
    let recurring_txns = match recurring_transaction::Entity::find()
        .filter(recurring_transaction::Column::CategoryId.is_not_null())
        .filter(recurring_transaction::Column::TargetAccountId.is_in(account_ids.clone()))
        .filter(recurring_transaction::Column::IsSimulated.eq(false))
        .all(db)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            let _ = writeln!(out, "Error loading recurring transactions: {}\n", e);
            return;
        }
    };

    // Load instances for overrides
    let instances = match recurring_transaction_instance::Entity::find()
        .filter(
            recurring_transaction_instance::Column::DueDate.between(range_start, today),
        )
        .all(db)
        .await
    {
        Ok(i) => i,
        Err(e) => {
            let _ = writeln!(out, "Error loading instances: {}\n", e);
            return;
        }
    };

    let instance_map: HashMap<(i32, NaiveDate), &recurring_transaction_instance::Model> =
        instances
            .iter()
            .map(|inst| ((inst.recurring_transaction_id, inst.due_date), inst))
            .collect();

    let category_map: HashMap<i32, &category::Model> =
        categories.iter().map(|c| (c.id, c)).collect();

    // Aggregate: category_id -> (yearly BTreeMap<year, Decimal>, count)
    let mut stats_map: HashMap<i32, (BTreeMap<i32, Decimal>, i64)> = HashMap::new();

    for txn in &one_off_txns {
        if let Some(category_id) = txn.category_id {
            let year = txn.date.year();
            let entry = stats_map
                .entry(category_id)
                .or_insert_with(|| (BTreeMap::new(), 0));
            *entry.0.entry(year).or_insert(Decimal::ZERO) += txn.amount;
            entry.1 += 1;
        }
    }

    for rtxn in &recurring_txns {
        let occurrences = generate_occurrences(
            rtxn.start_date,
            rtxn.end_date,
            &rtxn.period,
            range_start,
            today,
        );

        for date in occurrences {
            let (amount, cat_id) = if let Some(instance) = instance_map.get(&(rtxn.id, date)) {
                if instance.status == recurring_transaction_instance::InstanceStatus::Skipped {
                    continue;
                }
                let amount = instance.paid_amount.unwrap_or(instance.expected_amount);
                let cat = instance.category_id.or(rtxn.category_id);
                (amount, cat)
            } else {
                (rtxn.amount, rtxn.category_id)
            };

            if let Some(category_id) = cat_id {
                let year = date.year();
                let entry = stats_map
                    .entry(category_id)
                    .or_insert_with(|| (BTreeMap::new(), 0));
                *entry.0.entry(year).or_insert(Decimal::ZERO) += amount;
                entry.1 += 1;
            }
        }
    }

    // Build children map for tree propagation
    let mut children_map: HashMap<i32, Vec<i32>> = HashMap::new();
    for cat in categories {
        if let Some(parent_id) = cat.parent_id {
            children_map.entry(parent_id).or_default().push(cat.id);
        }
    }

    // Topological sort leaves-first
    let all_cat_ids: Vec<i32> = categories.iter().map(|c| c.id).collect();
    let topo_order = topological_sort_leaves_first(&all_cat_ids, &children_map);

    let mut tree_yearly: HashMap<i32, BTreeMap<i32, Decimal>> = HashMap::new();
    for cat in categories {
        tree_yearly.insert(
            cat.id,
            stats_map
                .get(&cat.id)
                .map(|(y, _)| y.clone())
                .unwrap_or_default(),
        );
    }

    for &cat_id in &topo_order {
        if let Some(parent_id) = category_map.get(&cat_id).and_then(|c| c.parent_id) {
            let child_yearly = tree_yearly.get(&cat_id).cloned().unwrap_or_default();
            let parent_yearly = tree_yearly.entry(parent_id).or_default();
            for (year, amount) in &child_yearly {
                *parent_yearly.entry(*year).or_insert(Decimal::ZERO) += amount;
            }
        }
    }

    let num_years = (range_start.year()..=today.year()).count() as i64;
    let num_years_dec = Decimal::from(num_years.max(1));

    // Collect only root categories or categories with data, sorted by total
    let mut category_stats: Vec<(String, Decimal, BTreeMap<i32, Decimal>)> = categories
        .iter()
        .filter(|c| c.parent_id.is_none())
        .filter_map(|cat| {
            let yearly = tree_yearly.get(&cat.id)?;
            let total: Decimal = yearly.values().copied().sum();
            if total == Decimal::ZERO {
                return None;
            }
            Some((cat.name.clone(), total / num_years_dec, yearly.clone()))
        })
        .collect();

    category_stats.sort_by(|a, b| a.1.cmp(&b.1));

    if category_stats.is_empty() {
        out.push_str("No categorized transactions found.\n\n");
        return;
    }

    let years: Vec<i32> = (range_start.year()..=today.year()).collect();

    out.push_str("| Category | Avg/Year |");
    for y in &years {
        let _ = write!(out, " {} |", y);
    }
    out.push('\n');

    out.push_str("|----------|----------|");
    for _ in &years {
        out.push_str("--------|");
    }
    out.push('\n');

    for (name, avg, yearly) in &category_stats {
        let _ = write!(out, "| {} | {} |", name, avg.round_dp(0));
        for y in &years {
            let val = yearly
                .get(y)
                .map(|d| d.round_dp(0).to_string())
                .unwrap_or_else(|| "-".into());
            let _ = write!(out, " {} |", val);
        }
        out.push('\n');
    }
    out.push('\n');

    // Also show child categories that have significant spending
    let child_stats: Vec<(String, String, Decimal)> = categories
        .iter()
        .filter(|c| c.parent_id.is_some())
        .filter_map(|cat| {
            let yearly = tree_yearly.get(&cat.id)?;
            let total: Decimal = yearly.values().copied().sum();
            if total == Decimal::ZERO {
                return None;
            }
            let parent_name = cat
                .parent_id
                .and_then(|pid| category_map.get(&pid))
                .map(|p| p.name.as_str())
                .unwrap_or("?");
            Some((
                format!("  {} > {}", parent_name, cat.name),
                parent_name.to_string(),
                total / num_years_dec,
            ))
        })
        .collect();

    if !child_stats.is_empty() {
        out.push_str("### Sub-categories (average per year)\n\n");
        out.push_str("| Category | Avg/Year |\n");
        out.push_str("|----------|----------|\n");
        for (name, _, avg) in &child_stats {
            let _ = writeln!(out, "| {} | {} |", name, avg.round_dp(0));
        }
        out.push('\n');
    }
}

fn write_user_prompt(out: &mut String) {
    out.push_str(r#"<user>
Based on all the financial data above, please:

1. **Assess my current financial situation** — What is my net worth? How healthy are my finances?
   Where am I heading based on the trends?

2. **Account organization** — Are my funds well-organized or scattered across too many accounts?
   Is each account serving a clear purpose? Should I consolidate anything?

3. **Goal review** — For accounts with target amounts, am I on track? Are the targets realistic?
   Are there accounts that should have goals but don't?

4. **12-month goals** — What specific, measurable targets should I aim for in the next 12 months?

5. **24-month goals** — What should my financial picture look like in 2 years?

6. **36-month goals** — What is a realistic 3-year vision?

7. **Priority actions** — What are the top 3-5 things I should do right now to improve my
   financial trajectory?
</user>
"#);
}

fn topological_sort_leaves_first(
    all_ids: &[i32],
    children_map: &HashMap<i32, Vec<i32>>,
) -> Vec<i32> {
    let mut result = Vec::with_capacity(all_ids.len());
    let mut visited: HashMap<i32, bool> = HashMap::new();

    fn visit(
        id: i32,
        children_map: &HashMap<i32, Vec<i32>>,
        visited: &mut HashMap<i32, bool>,
        result: &mut Vec<i32>,
    ) {
        if visited.contains_key(&id) {
            return;
        }
        visited.insert(id, true);
        if let Some(children) = children_map.get(&id) {
            for &child in children {
                visit(child, children_map, visited, result);
            }
        }
        result.push(id);
    }

    for &id in all_ids {
        visit(id, children_map, &mut visited, &mut result);
    }
    result
}

fn get_last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
}
