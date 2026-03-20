use anyhow::Result;
use chrono::{Datelike, Months, NaiveDate, Utc};
use common::metrics::{AccountKindMetricsDto, AccountMetricsDto, DashboardMetricsDto};
use compute::account::utils::generate_occurrences;
use compute::metrics::cross_account_metrics;
use compute::{account::AccountStateCalculator, account_stats, default_compute};
use model::entities::{
    account::{self, AccountKind},
    category, one_off_transaction, recurring_income, recurring_transaction,
    recurring_transaction_instance,
};
use polars::prelude::*;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, Condition, Database, DatabaseConnection, EntityTrait, QueryFilter};
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
    let mut prompt = String::with_capacity(32_000);

    write_system_prompt(&mut prompt);

    // Dashboard-level metrics
    let dashboard = cross_account_metrics::compute_dashboard_metrics(
        &compute as &dyn AccountStateCalculator,
        db,
        today,
    )
        .await
        .ok();
    if let Some(ref d) = dashboard {
        write_dashboard_metrics(&mut prompt, d, today);
    }

    // Per-account detail blocks with balances + forecast
    let account_metrics_map: HashMap<i32, &AccountMetricsDto> = dashboard
        .as_ref()
        .map(|d| d.account_metrics.iter().map(|m| (m.account_id, m)).collect())
        .unwrap_or_default();

    write_account_details(
        &mut prompt,
        &compute,
        db,
        &accounts,
        &account_metrics_map,
        today,
        months_back,
    )
        .await;

    // Recurring commitments
    write_recurring_commitments(&mut prompt, db, &accounts, &categories, today).await;

    // Income summary
    write_income_summary(&mut prompt, db, &accounts, today, months_back).await;

    // Category breakdown
    write_category_breakdown(&mut prompt, db, &accounts, &categories, today, months_back).await;

    write_user_prompt(&mut prompt);

    Ok(prompt)
}

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------

fn write_system_prompt(out: &mut String) {
    out.push_str(
        r#"<system>
You are the world's best financial advisor with decades of experience in personal finance,
wealth management, and financial planning. You are given structured financial data about a
person's accounts, monthly balances, income, and spending by category.

DATA FORMAT NOTES:
- All monetary amounts are in the account's currency (see currency_code per account).
- Negative amounts = outflows/spending. Positive amounts = inflows/income.
- Negative balances on Debt accounts represent outstanding debt.
- Monthly balances are end-of-month snapshots. Months after today's date are FORECAST
  (projected from recurring transactions and income).
- Category spending: negative values are expenses, positive values are income.
  The "Avg/Year" column is the sum of all transactions in that category across all years,
  divided by the number of years. Sub-categories show "Parent > Child" with their own
  average. Parent category totals INCLUDE child category totals (tree-aggregated).
- Account kinds: RealAccount (checking/operating), Savings, Investment, Debt, Goal,
  EmergencyFund, Allowance (personal spending), Shared (joint), Equity, House, Tax, Other.
- is_liquid = whether the account can be quickly converted to cash.
- Dashboard metrics explained:
  - essential_burn_rate: monthly cost of mandatory expenses (from operating accounts only)
  - full_burn_rate: total monthly expenses across all accounts
  - free_cashflow: net income minus full burn rate (what's left over each month)
  - savings_rate: free_cashflow / total_income (fraction saved)
  - goal_engine: monthly net inflow going toward wealth-building accounts
  - commitment_ratio: fixed recurring expenses / net income
  - liquidity_ratio_months: liquid assets / essential_burn_rate (runway in months)
  - total_debt_burden: monthly debt payments / net income

YOUR TASK:
1. Assess the current financial situation holistically
2. Analyze the trajectory — are things improving or deteriorating?
3. Check if money is scattered across too many accounts without clear purpose
4. Review goals (target amounts) and whether they are realistic given current flows
5. Suggest concrete, measurable goals for 12, 24, and 36 months
6. Provide actionable recommendations prioritized by impact
7. Flag any risks or concerns you see
8. Comment on the forecast — is the projected trajectory sustainable?
</system>

"#,
    );
}

// ---------------------------------------------------------------------------
// Dashboard metrics
// ---------------------------------------------------------------------------

fn write_dashboard_metrics(out: &mut String, d: &DashboardMetricsDto, today: NaiveDate) {
    out.push_str("## FINANCIAL HEALTH DASHBOARD\n\n");
    let _ = writeln!(out, "As of: {}\n", today);

    out.push_str("| Metric | Value |\n");
    out.push_str("|--------|-------|\n");
    let _ = writeln!(out, "| Total Net Worth | {} |", d.total_net_worth.round_dp(0));
    let _ = writeln!(
        out,
        "| Liquid Net Worth | {} |",
        d.liquid_net_worth.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| Non-Liquid Net Worth | {} |",
        d.non_liquid_net_worth.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| Essential Burn Rate (monthly) | {} |",
        d.essential_burn_rate.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| Full Burn Rate (monthly) | {} |",
        d.full_burn_rate.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| Free Cashflow (monthly) | {} |",
        d.free_cashflow.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| Savings Rate | {} |",
        fmt_pct(d.savings_rate)
    );
    let _ = writeln!(
        out,
        "| Goal Engine (monthly wealth-building) | {} |",
        d.goal_engine.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| Commitment Ratio | {} |",
        fmt_pct(d.commitment_ratio)
    );
    let _ = writeln!(
        out,
        "| Liquidity Ratio | {} |",
        d.liquidity_ratio_months
            .map(|v| format!("{:.1} months", v))
            .unwrap_or_else(|| "N/A".into())
    );
    let _ = writeln!(
        out,
        "| Total Debt Burden | {} |",
        fmt_pct(d.total_debt_burden)
    );
    out.push('\n');
}

// ---------------------------------------------------------------------------
// Per-account detail blocks
// ---------------------------------------------------------------------------

async fn write_account_details(
    out: &mut String,
    compute: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    metrics_map: &HashMap<i32, &AccountMetricsDto>,
    today: NaiveDate,
    months_back: u32,
) {
    out.push_str("## ACCOUNT DETAILS\n\n");

    let range_start = today
        .checked_sub_months(Months::new(months_back))
        .unwrap_or(today)
        .with_day(1)
        .unwrap_or(today);
    let year_end = NaiveDate::from_ymd_opt(today.year(), 12, 31).unwrap();

    // Compute balances per-account using two separate calls (historical + forecast)
    // matching the dashboard approach to avoid accumulation bugs with long ranges.
    let mut all_balances: HashMap<i32, HashMap<String, Decimal>> = HashMap::new();
    for a in accounts {
        let single = vec![a.clone()];
        let mut account_months: HashMap<String, Decimal> = HashMap::new();

        // Historical: range_start → today
        if let Ok(df) = compute
            .compute_account_state(db, &single, range_start, today)
            .await
        {
            if let Ok(months) = extract_last_per_month_balances(&df) {
                account_months.extend(months);
            }
        }

        // Forecast: today → year_end
        if today < year_end {
            if let Ok(df) = compute
                .compute_account_state(db, &single, today, year_end)
                .await
            {
                if let Ok(months) = extract_last_per_month_balances(&df) {
                    account_months.extend(months);
                }
            }
        }

        all_balances.insert(a.id, account_months);
    }

    // Build month labels
    let month_labels = build_month_labels(range_start, year_end);

    // Per-account year stats
    let year = today.year();
    let year_stats = compute_year_stats(compute, db, accounts, year).await;

    for a in accounts {
        let _ = writeln!(out, "### {}\n", a.name);

        // Properties
        out.push_str("**Properties:**\n");
        let _ = writeln!(out, "- Type: {}", account_kind_str(a.account_kind));
        let _ = writeln!(out, "- Currency: {}", a.currency_code);
        let _ = writeln!(
            out,
            "- Target Amount: {}",
            a.target_amount
                .map(|t| t.round_dp(0).to_string())
                .unwrap_or_else(|| "None".into())
        );
        let _ = writeln!(
            out,
            "- Liquid: {}",
            if a.is_liquid { "Yes" } else { "No" }
        );
        if let Some(desc) = &a.description {
            if !desc.is_empty() {
                let _ = writeln!(out, "- Description: {}", desc);
            }
        }
        out.push('\n');

        // Metrics from dashboard
        if let Some(m) = metrics_map.get(&a.id) {
            write_account_metrics(out, m);
        }

        // Year statistics
        if let Some(stats) = year_stats.get(&a.id) {
            write_account_year_stats(out, stats, year);
        }

        // Monthly balances + forecast
        if let Some(account_months) = all_balances.get(&a.id) {
            write_account_balance_table(out, account_months, &month_labels, today);
        }

        out.push('\n');
    }
}

fn write_account_metrics(out: &mut String, m: &AccountMetricsDto) {
    out.push_str("**Current Metrics:**\n");
    let _ = writeln!(
        out,
        "- Current Balance: {}",
        m.current_balance.round_dp(0)
    );
    if let Some(t) = m.target_balance {
        let _ = writeln!(out, "- Target Balance: {}", t.round_dp(0));
    }
    if let Some(f) = m.funding_ratio {
        let _ = writeln!(out, "- Funding Ratio: {}", fmt_pct(Some(f)));
    }
    if let Some(f) = m.monthly_net_flow {
        let _ = writeln!(out, "- Monthly Net Flow: {}", f.round_dp(0));
    }
    if let Some(f) = m.three_month_avg_net_flow {
        let _ = writeln!(out, "- 3-Month Avg Net Flow: {}", f.round_dp(0));
    }
    if let Some(v) = m.flow_volatility {
        let _ = writeln!(out, "- Flow Volatility (stddev): {}", v.round_dp(0));
    }

    if let Some(kind) = &m.kind_metrics {
        match kind {
            AccountKindMetricsDto::Operating(op) => {
                if let Some(b) = op.operating_buffer {
                    let _ = writeln!(out, "- Operating Buffer: {}", b.round_dp(0));
                }
                if let Some(s) = op.sweep_potential {
                    let _ = writeln!(out, "- Sweep Potential: {}", s.round_dp(0));
                }
                if let Some(c) = op.mandatory_coverage_months {
                    let _ = writeln!(out, "- Mandatory Coverage: {:.1} months", c);
                }
            }
            AccountKindMetricsDto::Reserve(res) => {
                if let Some(d) = res.goal_reached_date {
                    let _ = writeln!(out, "- Goal Reached Date: {}", d);
                }
                if let Some(c) = res.months_of_essential_coverage {
                    let _ = writeln!(out, "- Essential Coverage: {:.1} months", c);
                }
            }
            AccountKindMetricsDto::Investment(inv) => {
                if let Some(nc) = inv.net_contributions {
                    let _ = writeln!(out, "- Net Contributions: {}", nc.round_dp(0));
                }
                if let Some(gl) = inv.gain_loss_absolute {
                    let _ = writeln!(out, "- Gain/Loss: {}", gl.round_dp(0));
                }
                if let Some(pct) = inv.gain_loss_percent {
                    let _ = writeln!(out, "- Return: {:.1}%", pct);
                }
            }
            AccountKindMetricsDto::Debt(debt) => {
                if let Some(p) = debt.outstanding_principal {
                    let _ = writeln!(out, "- Outstanding Principal: {}", p.round_dp(0));
                }
                if let Some(p) = debt.required_monthly_payment {
                    let _ = writeln!(out, "- Required Monthly Payment: {}", p.round_dp(0));
                }
                if let Some(d) = debt.debt_free_date {
                    let _ = writeln!(out, "- Debt-Free Date: {}", d);
                }
            }
        }
    }
    out.push('\n');
}

fn write_account_year_stats(out: &mut String, stats: &account_stats::AccountStats, year: i32) {
    out.push_str(&format!("**Statistics ({}):**\n", year));
    if let Some(v) = stats.min_state {
        let _ = writeln!(out, "- Min Balance: {}", v.round_dp(0));
    }
    if let Some(v) = stats.max_state {
        let _ = writeln!(out, "- Max Balance: {}", v.round_dp(0));
    }
    if let Some(v) = stats.average_expense {
        let _ = writeln!(out, "- Avg Monthly Expense: {}", v.round_dp(0));
    }
    if let Some(v) = stats.average_income {
        let _ = writeln!(out, "- Avg Monthly Income: {}", v.round_dp(0));
    }
    if let Some(v) = stats.upcoming_expenses {
        let _ = writeln!(out, "- Upcoming Expenses (rest of year): {}", v.round_dp(0));
    }
    if let Some(v) = stats.end_of_period_state {
        let _ = writeln!(out, "- Projected Year-End Balance: {}", v.round_dp(0));
    }
    out.push('\n');
}

fn write_account_balance_table(
    out: &mut String,
    account_months: &HashMap<String, Decimal>,
    month_labels: &[String],
    today: NaiveDate,
) {
    let today_label = format!("{}-{:02}", today.year(), today.month());
    out.push_str("**Monthly Balances:**\n\n");
    out.push_str("| Month | Balance | Note |\n");
    out.push_str("|-------|---------|------|\n");
    for label in month_labels {
        if let Some(bal) = account_months.get(label) {
            let note = if label > &today_label {
                "forecast"
            } else if label == &today_label {
                "current"
            } else {
                ""
            };
            let _ = writeln!(out, "| {} | {} | {} |", label, bal.round_dp(0), note);
        }
    }
    out.push('\n');
}

// ---------------------------------------------------------------------------
// Recurring commitments
// ---------------------------------------------------------------------------

async fn write_recurring_commitments(
    out: &mut String,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    categories: &[category::Model],
    today: NaiveDate,
) {
    out.push_str("## ACTIVE RECURRING COMMITMENTS\n\n");

    let recurring_txns: Vec<recurring_transaction::Model> =
        match recurring_transaction::Entity::find()
            .filter(recurring_transaction::Column::IsSimulated.eq(false))
            .filter(recurring_transaction::Column::StartDate.lte(today))
            .filter(
                Condition::any()
                    .add(recurring_transaction::Column::EndDate.is_null())
                    .add(recurring_transaction::Column::EndDate.gte(today)),
            )
            .all(db)
            .await
        {
            Ok(t) => t,
            Err(e) => {
                let _ = writeln!(out, "Error loading recurring: {}\n", e);
                return;
            }
        };

    if recurring_txns.is_empty() {
        out.push_str("No active recurring transactions.\n\n");
        return;
    }

    let account_map: HashMap<i32, &account::Model> =
        accounts.iter().map(|a| (a.id, a)).collect();
    let category_map: HashMap<i32, &category::Model> =
        categories.iter().map(|c| (c.id, c)).collect();

    out.push_str("| Name | Amount | Period | Account | Category |\n");
    out.push_str("|------|--------|--------|---------|----------|\n");

    let mut sorted = recurring_txns.clone();
    sorted.sort_by(|a, b| a.amount.cmp(&b.amount));

    for t in &sorted {
        let acct = account_map
            .get(&t.target_account_id)
            .map(|a| a.name.as_str())
            .unwrap_or("-");
        let cat = t
            .category_id
            .and_then(|cid| category_map.get(&cid))
            .map(|c| c.name.as_str())
            .unwrap_or("-");
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            t.name,
            t.amount.round_dp(0),
            period_str(&t.period),
            acct,
            cat,
        );
    }
    out.push('\n');
}

// ---------------------------------------------------------------------------
// Income summary (kept similar, small tweaks)
// ---------------------------------------------------------------------------

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

    let mut yearly_totals: BTreeMap<i32, Decimal> = BTreeMap::new();
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

        per_income.push((income.name.clone(), acct_name.to_string(), income_yearly));
    }

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

// ---------------------------------------------------------------------------
// Category breakdown (fixed sub-categories)
// ---------------------------------------------------------------------------

async fn write_category_breakdown(
    out: &mut String,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    categories: &[category::Model],
    today: NaiveDate,
    months_back: u32,
) {
    out.push_str("## SPENDING & INCOME BY CATEGORY\n\n");
    out.push_str("_Negative = spending/outflow. Positive = income/inflow. ");
    out.push_str("Avg/Year is the total across all years divided by the number of years. ");
    out.push_str("Parent categories include all child category amounts (tree-aggregated)._\n\n");

    let range_start = today
        .checked_sub_months(Months::new(months_back))
        .unwrap_or(today)
        .with_day(1)
        .unwrap_or(today);

    let account_ids: Vec<i32> = accounts.iter().map(|a| a.id).collect();

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

    let instances = match recurring_transaction_instance::Entity::find()
        .filter(recurring_transaction_instance::Column::DueDate.between(range_start, today))
        .all(db)
        .await
    {
        Ok(i) => i,
        Err(e) => {
            let _ = writeln!(out, "Error loading instances: {}\n", e);
            return;
        }
    };

    let instance_map: HashMap<(i32, NaiveDate), &recurring_transaction_instance::Model> = instances
        .iter()
        .map(|inst| ((inst.recurring_transaction_id, inst.due_date), inst))
        .collect();

    let category_map: HashMap<i32, &category::Model> =
        categories.iter().map(|c| (c.id, c)).collect();

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

    // Tree propagation
    let mut children_map: HashMap<i32, Vec<i32>> = HashMap::new();
    for cat in categories {
        if let Some(parent_id) = cat.parent_id {
            children_map.entry(parent_id).or_default().push(cat.id);
        }
    }

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
    let num_months = Decimal::from((num_years * 12).max(1));

    // Root categories
    let mut category_stats: Vec<(String, Decimal, Decimal, BTreeMap<i32, Decimal>)> = categories
        .iter()
        .filter(|c| c.parent_id.is_none())
        .filter_map(|cat| {
            let yearly = tree_yearly.get(&cat.id)?;
            let total: Decimal = yearly.values().copied().sum();
            if total == Decimal::ZERO {
                return None;
            }
            Some((
                cat.name.clone(),
                total / num_years_dec,
                total / num_months,
                yearly.clone(),
            ))
        })
        .collect();

    category_stats.sort_by(|a, b| a.1.cmp(&b.1));

    if category_stats.is_empty() {
        out.push_str("No categorized transactions found.\n\n");
        return;
    }

    let years: Vec<i32> = (range_start.year()..=today.year()).collect();

    out.push_str("| Category | Avg/Year | Avg/Month |");
    for y in &years {
        let _ = write!(out, " {} |", y);
    }
    out.push('\n');

    out.push_str("|----------|----------|-----------|");
    for _ in &years {
        out.push_str("--------|");
    }
    out.push('\n');

    for (name, avg_yr, avg_mo, yearly) in &category_stats {
        let _ = write!(out, "| {} | {} | {} |", name, avg_yr.round_dp(0), avg_mo.round_dp(0));
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

    // Sub-categories: show own direct amounts only, sorted by absolute avg
    let mut child_stats: Vec<(String, Decimal, Decimal)> = categories
        .iter()
        .filter(|c| c.parent_id.is_some())
        .filter_map(|cat| {
            let own_yearly = stats_map.get(&cat.id)?;
            let total: Decimal = own_yearly.0.values().copied().sum();
            if total == Decimal::ZERO {
                return None;
            }
            let parent_name = cat
                .parent_id
                .and_then(|pid| category_map.get(&pid))
                .map(|p| p.name.as_str())
                .unwrap_or("?");
            Some((
                format!("{} > {}", parent_name, cat.name),
                total / num_years_dec,
                total / num_months,
            ))
        })
        .collect();

    child_stats.sort_by(|a, b| a.1.cmp(&b.1));

    if !child_stats.is_empty() {
        out.push_str("### Sub-category breakdown\n\n");
        out.push_str("_These are direct (own) amounts per sub-category, NOT including their children._\n\n");
        out.push_str("| Sub-Category | Avg/Year | Avg/Month |\n");
        out.push_str("|--------------|----------|----------|\n");
        for (name, avg_yr, avg_mo) in &child_stats {
            let _ = writeln!(
                out,
                "| {} | {} | {} |",
                name,
                avg_yr.round_dp(0),
                avg_mo.round_dp(0)
            );
        }
        out.push('\n');
    }
}

// ---------------------------------------------------------------------------
// User prompt
// ---------------------------------------------------------------------------

fn write_user_prompt(out: &mut String) {
    out.push_str(
        r#"<user>
Based on all the financial data above, please:

1. **Assess my current financial situation** — What is my net worth? How healthy are my finances?
   Where am I heading based on the trends? How does the forecast look?

2. **Account organization** — Are my funds well-organized or scattered across too many accounts?
   Is each account serving a clear purpose? Should I consolidate anything?

3. **Cash flow analysis** — Is my spending sustainable? What is my effective savings rate?
   Are there accounts bleeding money? Comment on the burn rate vs income.

4. **Goal review** — For accounts with target amounts, am I on track? Are the targets realistic?
   Are there accounts that should have goals but don't?

5. **12-month goals** — What specific, measurable targets should I aim for in the next 12 months?

6. **24-month goals** — What should my financial picture look like in 2 years?

7. **36-month goals** — What is a realistic 3-year vision?

8. **Priority actions** — What are the top 3-5 things I should do right now to improve my
   financial trajectory?

9. **Risk assessment** — What are the biggest financial risks I face? Do I have enough
   emergency reserves? Is my liquidity ratio healthy?
</user>
"#,
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn period_str(period: &recurring_transaction::RecurrencePeriod) -> &'static str {
    use recurring_transaction::RecurrencePeriod::*;
    match period {
        Daily => "Daily",
        Weekly => "Weekly",
        WorkDay => "Work Days",
        Monthly => "Monthly",
        Quarterly => "Quarterly",
        HalfYearly => "Half-Yearly",
        Yearly => "Yearly",
    }
}

fn fmt_pct(v: Option<Decimal>) -> String {
    v.map(|d| format!("{:.1}%", d * Decimal::from(100)))
        .unwrap_or_else(|| "N/A".into())
}

fn build_month_labels(start: NaiveDate, end: NaiveDate) -> Vec<String> {
    let mut labels = Vec::new();
    let mut cursor = start.with_day(1).unwrap_or(start);
    while cursor <= end {
        labels.push(format!("{}-{:02}", cursor.year(), cursor.month()));
        cursor = cursor
            .checked_add_months(Months::new(1))
            .unwrap_or(end);
        if cursor <= start {
            break;
        }
    }
    labels.sort();
    labels.dedup();
    labels
}

/// Extract the last balance per month from a DataFrame.
///
/// Groups data points by year-month and takes the last (latest date) balance
/// for each month. This mirrors how the dashboard charts display balances.
fn extract_last_per_month_balances(
    df: &DataFrame,
) -> std::result::Result<HashMap<String, Decimal>, String> {
    let date_col = df
        .column("date")
        .map_err(|e| format!("Missing date column: {e}"))?;
    let balance_col = df
        .column("balance")
        .map_err(|e| format!("Missing balance column: {e}"))?;

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let mut month_data: HashMap<String, (i64, Decimal)> = HashMap::new();

    for i in 0..df.height() {
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

        let month_key = format!("{}-{:02}", date.year(), date.month());
        match month_data.get(&month_key) {
            Some((existing_date, _)) if date_epoch > *existing_date => {
                month_data.insert(month_key, (date_epoch, bal));
            }
            None => {
                month_data.insert(month_key, (date_epoch, bal));
            }
            _ => {}
        }
    }

    Ok(month_data.into_iter().map(|(k, (_, bal))| (k, bal)).collect())
}

/// Compute year-level statistics for all accounts in one pass.
async fn compute_year_stats(
    compute: &dyn AccountStateCalculator,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    year: i32,
) -> HashMap<i32, account_stats::AccountStats> {
    let mut result: HashMap<i32, account_stats::AccountStats> = HashMap::new();

    // Initialize empty stats for all accounts
    for a in accounts {
        result.insert(
            a.id,
            account_stats::AccountStats {
                account_id: a.id,
                min_state: None,
                max_state: None,
                average_expense: None,
                average_income: None,
                upcoming_expenses: None,
                end_of_period_state: None,
            },
        );
    }

    if let Ok(min) = account_stats::min_state_in_year(compute, db, accounts, year).await {
        for s in min {
            if let Some(entry) = result.get_mut(&s.account_id) {
                entry.min_state = s.min_state;
            }
        }
    }
    if let Ok(max) = account_stats::max_state_in_year(compute, db, accounts, year).await {
        for s in max {
            if let Some(entry) = result.get_mut(&s.account_id) {
                entry.max_state = s.max_state;
            }
        }
    }
    if let Ok(avg_exp) = account_stats::average_expense_in_year(compute, db, accounts, year).await
    {
        for s in avg_exp {
            if let Some(entry) = result.get_mut(&s.account_id) {
                entry.average_expense = s.average_expense;
            }
        }
    }
    if let Ok(avg_inc) = account_stats::average_income_in_year(compute, db, accounts, year).await {
        for s in avg_inc {
            if let Some(entry) = result.get_mut(&s.account_id) {
                entry.average_income = s.average_income;
            }
        }
    }
    let today = Utc::now().date_naive();
    if let Ok(upcoming) =
        account_stats::upcoming_expenses_until_year_end(compute, db, accounts, year, today).await
    {
        for s in upcoming {
            if let Some(entry) = result.get_mut(&s.account_id) {
                entry.upcoming_expenses = s.upcoming_expenses;
            }
        }
    }
    if let Ok(eoy) = account_stats::end_of_year_state(compute, db, accounts, year).await {
        for s in eoy {
            if let Some(entry) = result.get_mut(&s.account_id) {
                entry.end_of_period_state = s.end_of_period_state;
            }
        }
    }

    result
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
