use anyhow::Result;
use chrono::{Datelike, Months, NaiveDate, Utc};
use common::metrics::{AccountKindMetricsDto, AccountMetricsDto, DashboardMetricsDto};
use compute::account::utils::generate_occurrences;
use compute::metrics::account_role::derive_account_role as compute_account_role;
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

    // Shock readiness
    if let Some(ref d) = dashboard {
        write_shock_readiness(&mut prompt, d, &accounts);
    }

    // Mortgage refix readiness
    write_mortgage_refix_readiness(&mut prompt, &accounts, &account_metrics_map);

    // Goal engine split (safety / consumption / wealth)
    if let Some(ref d) = dashboard {
        write_goal_engine_split(&mut prompt, &accounts, &d.account_metrics);
    }

    // Recent financial changes
    write_recent_financial_changes(&mut prompt, db, &accounts, today).await;

    // Known future events
    write_known_future_events(&mut prompt, &accounts);

    write_user_prompt(&mut prompt);

    Ok(prompt)
}

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------

fn write_system_prompt(out: &mut String) {
    out.push_str(
        r#"<system>
You are a rigorous, skeptical financial advisor specializing in household cashflow safety,
liquidity management, and behavioral finance. You are given structured financial data about
a family's accounts, monthly balances, income, and spending by category.

CRITICAL MINDSET RULES:
- Be skeptical of dashboard metrics and forecasts. Do not describe the situation as "strong"
  or "healthy" if liquidity, emergency reserves, or cashflow discipline are weak.
- Prioritize LIQUIDITY, RUNWAY, and CONTROLLABLE CASHFLOW over total net worth.
- Treat home equity as non-operating wealth unless the user explicitly plans to sell or refinance.
- Distinguish clearly between:
  * true wealth building (long-term investments, retirement, extra principal payments)
  * emergency reserves (only money available for genuine emergencies)
  * sinking funds (vacation, car downpayment, annual bills — these WILL be spent)
  * income smoothing reserves (buffer for variable-income months)
  * tax reserves (committed liability, not savings)
  * internal transfers (account reshuffling that creates no new wealth)
- If assumptions are fragile, say so directly.
- If forecast success depends on new habits or strict discipline, call that out explicitly.
- Do NOT inflate free cashflow by counting internal transfers or sinking fund contributions as "saved."

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
- Account purpose metadata: each account has a "role" field describing its structural type
  and boolean flags indicating whether it counts as emergency reserve, income smoothing, etc.
  IMPORTANT: These flags are derived from the account kind only. You MUST also read each
  account's description to refine your interpretation. For example, an EmergencyFund account
  whose description says "house maintenance" is really a maintenance reserve, not a true
  emergency fund. A Savings account described as "vacation replacement / income buffer" is
  really an income smoothing reserve. Do NOT assume all "savings" accounts are building wealth.
- is_liquid = whether the account can be quickly converted to cash.
- Dashboard metrics explained:
  - essential_burn_rate: monthly cost of mandatory expenses (from operating accounts only)
  - full_burn_rate: total monthly expenses across all accounts
  - controllable_burn_rate: necessary but adjustable expenses (groceries, fuel, etc.) — not yet available
  - discretionary_burn_rate: optional lifestyle expenses — not yet available
  - free_cashflow: net income minus full burn rate — WARNING: this may include internal
    transfers and sinking fund allocations; see "operating_free_cashflow" for a cleaner number
  - operating_free_cashflow: operating net flow + transfers to TRUE WEALTH only (excludes
    sinking funds, tax reserves, allowances) — THIS IS THE REAL NUMBER
  - savings_rate: free_cashflow / total_income — WARNING: inflated if sinking funds counted
  - goal_engine: monthly net inflow to all wealth-like accounts (LEGACY METRIC - misleading)
  - safety_reserve_rate: monthly flow to emergency fund + income smoothing
  - consumption_goal_rate: monthly flow to sinking funds + allowances (will be spent)
  - wealth_building_rate: monthly flow to true long-term investments
  - debt_payment_rate: monthly debt payments (mandatory expenses, tracked separately)
  - savings_rate_category: monthly flow to Savings/Goal account kinds
  - NOTE: Tax reserves and debt payments are treated as mandatory spending, NOT counted in goal categories
  - NOTE: Savings/Goal accounts are tracked separately from consumption
  - commitment_ratio: fixed recurring expenses / net income
  - liquidity_ratio_months: liquid assets / essential_burn_rate (runway in months)
  - total_debt_burden: monthly debt payments / net income
  - shock_readiness_1m/3m/6m: whether true reserves cover 1/3/6 months of essential burn
- Burn rate layers (currently essential = full; others not yet classified by category):
  - Mandatory burn: mortgage, utilities, insurance, taxes, minimum debt payments, basic food, required transport
  - Controllable burn: groceries, lunches, fuel, household supplies — necessary but adjustable
  - Discretionary burn: gifts, hobbies, electronics, garden, streaming, family extras — optional
- Account role classification:
  - Each account has a BASE type (RealAccount, Savings, EmergencyFund, etc.)
  - AND a DERIVED semantic role based on description + metadata
  - The LLM MUST use the derived role + description, NOT just the base type
  - Example: Savings account described as "vacation fund" → derived role: sinking_fund (earmarked spending)
  - Example: Savings account described as "income buffer for OSVC" → derived role: income_smoothing

REQUIRED EVALUATION STRUCTURE (use these 4 layers in order):
1. LIQUIDITY / SAFETY: Are there real emergency reserves? How many months of disruption
   can the household survive? Is shock readiness adequate?
2. CASHFLOW QUALITY: Is operating free cashflow positive after removing internal transfers?
   What share of outflows are true wealth-building vs future spending allocations?
3. BEHAVIORAL LEAKAGE / SPENDING CONTROL: What are the top historical leak categories?
   Are control systems (allowance, shared account caps) proven or newly introduced?
   Is discretionary spending actually contained or just temporarily paused?
4. LONG-TERM WEALTH BUILDING: Is money going to real investments? Are debt paydowns on track?
   Is retirement funded? Is the household building or just treading water?

FORECAST CHALLENGE RULES:
- Separate MECHANICALLY PROJECTED outcomes from BEHAVIORALLY CREDIBLE outcomes.
- State whether forecast improvements are:
  * already proven in historical behavior
  * newly introduced but unproven
  * dependent on strict future discipline
- Highlight fragile assumptions explicitly.

LEAK DETECTION RULES:
- Identify top historical leak categories and quantify approximate yearly impact.
- State whether each leak is: structurally solved, temporarily paused, or still unresolved.
- Flag any discretionary spending that bypasses the control system (e.g., spending from
  main/operating accounts that should go through allowance accounts).

ACTION RECOMMENDATIONS FORMAT:
Provide recommendations in this priority order:
1. STOP — things to eliminate immediately
2. PAUSE — things to suspend until a condition is met
3. REDUCE — things to cut back
4. KEEP — things that are working well
5. INCREASE — things to do more of
6. CREATE — new habits or systems to establish
For each recommendation include: why it matters, monthly impact if known, and whether
it improves liquidity, discipline, long-term growth, or debt/risk reduction.
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
        "| ↳ Operating Net Flow | {} |",
        d.cashflow_breakdown.operating_net_flow.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| ↳ Committed Transfers Out | +{} |",
        d.cashflow_breakdown.committed_transfers_out.round_dp(0)
    );
    if let Some(ofc) = d.operating_free_cashflow {
        let _ = writeln!(
            out,
            "| **Operating Free Cashflow (REAL)** | **{}** |",
            ofc.round_dp(0)
        );
    }

    // Operating free cashflow breakdown
    if let Some(ref breakdown) = d.operating_free_cashflow_breakdown {
        let _ = writeln!(out, "|   |   |   |");
        let _ = writeln!(out, "| **Operating Cashflow Breakdown:** | **This Month** | **3-mo Avg** |");
        for contrib in &breakdown.contributions {
            let sign = if contrib.net_flow.is_sign_negative() { "" } else { "+" };
            let avg_text = if let Some(avg) = contrib.three_month_avg_net_flow {
                let avg_sign = if avg.is_sign_negative() { "" } else { "+" };
                format!("{}{}", avg_sign, avg.round_dp(0))
            } else {
                "N/A".to_string()
            };
            let _ = writeln!(
                out,
                "| ↳ {} ({}) | {}{} | {} |",
                contrib.account_name,
                contrib.account_kind,
                sign,
                contrib.net_flow.round_dp(0),
                avg_text
            );
        }
        let _ = writeln!(
            out,
            "| = **Total Operating Cashflow** | **{}** | |",
            breakdown.total.round_dp(0)
        );
        let _ = writeln!(out, "|   |   |   |");
    }
    let _ = writeln!(
        out,
        "| Savings Rate | {} |",
        fmt_pct(d.savings_rate)
    );
    let _ = writeln!(
        out,
        "| Goal Engine (LEGACY - misleading) | {} |",
        d.goal_engine.round_dp(0)
    );
    if let Some(sr) = d.safety_reserve_rate {
        let _ = writeln!(
            out,
            "| ↳ Safety Reserve Rate | {} |",
            sr.round_dp(0)
        );
    }
    if let Some(cr) = d.consumption_goal_rate {
        let _ = writeln!(
            out,
            "| ↳ Consumption Goal Rate | {} |",
            cr.round_dp(0)
        );
    }
    if let Some(wr) = d.wealth_building_rate {
        let _ = writeln!(
            out,
            "| ↳ Wealth Building Rate | {} |",
            wr.round_dp(0)
        );
    }
    if let Some(dr) = d.debt_payment_rate {
        let _ = writeln!(
            out,
            "| ↳ Debt Payment Rate | {} |",
            dr.round_dp(0)
        );
    }
    if let Some(sr) = d.savings_rate_category {
        let _ = writeln!(
            out,
            "| ↳ Savings Rate | {} |",
            sr.round_dp(0)
        );
    }
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
    if let Some(sr1) = d.shock_readiness_1m {
        let _ = writeln!(
            out,
            "| Shock Readiness (1-month) | {} |",
            if sr1 { "YES" } else { "NO" }
        );
    }
    if let Some(sr3) = d.shock_readiness_3m {
        let _ = writeln!(
            out,
            "| Shock Readiness (3-month) | {} |",
            if sr3 { "YES" } else { "NO" }
        );
    }
    if let Some(sr6) = d.shock_readiness_6m {
        let _ = writeln!(
            out,
            "| Shock Readiness (6-month) | {} |",
            if sr6 { "YES" } else { "NO" }
        );
    }
    out.push('\n');

    // Cashflow breakdown: per-account contributions
    let _ = writeln!(
        out,
        "**Cashflow Breakdown** ({})\n",
        d.cashflow_breakdown.timeframe
    );
    let _ = writeln!(out, "{}\n", d.cashflow_breakdown.description);
    out.push_str("| Account | Kind | Net Flow |\n");
    out.push_str("|---------|------|----------|\n");
    for c in &d.cashflow_breakdown.contributions {
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            c.account_name,
            c.account_kind,
            c.net_flow.round_dp(0)
        );
    }
    let _ = writeln!(
        out,
        "| **Operating subtotal** | | **{}** |",
        d.cashflow_breakdown.operating_net_flow.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| **+ Committed transfers out** | | **+{}** |",
        d.cashflow_breakdown.committed_transfers_out.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| **= Free Cashflow** | | **{}** |",
        d.cashflow_breakdown.free_cashflow.round_dp(0)
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

        // Account role metadata (derived from type + description)
        let role = compute_account_role(a);
        out.push_str("**Account Role Metadata:**\n");
        let _ = writeln!(out, "- base_type: {}", account_kind_str(a.account_kind));
        let _ = writeln!(out, "- derived_role: {}", role.role);
        let _ = writeln!(out, "- classification_reason: {}", role.classification_reason);
        let _ = writeln!(out, "- purpose: {}", role.purpose);
        let _ = writeln!(
            out,
            "- can_be_used_in_emergency: {}",
            role.can_be_used_in_emergency
        );
        let _ = writeln!(
            out,
            "- counts_as_emergency_reserve: {}",
            role.counts_as_emergency_reserve
        );
        let _ = writeln!(
            out,
            "- counts_as_income_smoothing: {}",
            role.counts_as_income_smoothing
        );
        let _ = writeln!(
            out,
            "- counts_as_long_term_wealth: {}",
            role.counts_as_long_term_wealth
        );
        let _ = writeln!(
            out,
            "- is_earmarked_spending: {}",
            role.is_earmarked_spending
        );
        let _ = writeln!(out, "- priority_level: {}", role.priority_level);
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
// Shock readiness
// ---------------------------------------------------------------------------

fn write_shock_readiness(
    out: &mut String,
    d: &DashboardMetricsDto,
    accounts: &[account::Model],
) {
    out.push_str("## SHOCK READINESS\n\n");
    out.push_str("_How long can the household survive an income disruption using TRUE emergency reserves only?_\n\n");

    // Compute emergency reserve total (only true emergency fund accounts)
    let mut emergency_reserve = Decimal::ZERO;
    let mut income_smoothing_reserve = Decimal::ZERO;
    let mut operating_buffer = Decimal::ZERO;

    for a in accounts {
        let role = compute_account_role(a);
        let balance = d
            .account_metrics
            .iter()
            .find(|m| m.account_id == a.id)
            .map(|m| m.current_balance)
            .unwrap_or(Decimal::ZERO);

        if role.counts_as_emergency_reserve {
            emergency_reserve += balance;
        }
        if role.counts_as_income_smoothing {
            income_smoothing_reserve += balance;
        }
        if role.role == "operating" && balance > Decimal::ZERO {
            operating_buffer += balance;
        }
    }

    let essential_monthly = d.essential_burn_rate.abs();
    let total_available = operating_buffer + emergency_reserve + income_smoothing_reserve;

    out.push_str("| Resource | Amount |\n");
    out.push_str("|----------|--------|\n");
    let _ = writeln!(
        out,
        "| Operating account buffers | {} |",
        operating_buffer.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| Emergency fund (true) | {} |",
        emergency_reserve.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| Income smoothing reserve | {} |",
        income_smoothing_reserve.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| **Total available for disruption** | **{}** |",
        total_available.round_dp(0)
    );
    let _ = writeln!(
        out,
        "| Essential monthly burn | {} |",
        essential_monthly.round_dp(0)
    );
    out.push('\n');

    if essential_monthly > Decimal::ZERO {
        let coverage_months = total_available / essential_monthly;

        // Use dashboard-computed shock readiness if available
        let months_1 = d.shock_readiness_1m.map(|sr| if sr { "YES" } else { "NO" }).unwrap_or("N/A");
        let months_3 = d.shock_readiness_3m.map(|sr| if sr { "YES" } else { "NO" }).unwrap_or("N/A");
        let months_6 = d.shock_readiness_6m.map(|sr| if sr { "YES" } else { "NO" }).unwrap_or("N/A");

        let _ = writeln!(
            out,
            "- **1-month shock readiness**: {} (coverage: {:.1} months)",
            months_1, coverage_months
        );
        let _ = writeln!(out, "- **3-month shock readiness**: {}", months_3);
        let _ = writeln!(out, "- **6-month shock readiness**: {}", months_6);
    } else {
        out.push_str("- Essential burn rate is zero — cannot compute shock readiness.\n");
    }

    out.push_str("\n_Note: Uses ONLY true emergency reserves + operating buffers. Excludes tax reserves, sinking funds, investments, and house equity._\n\n");
}

// ---------------------------------------------------------------------------
// Mortgage refix readiness
// ---------------------------------------------------------------------------

fn write_mortgage_refix_readiness(
    out: &mut String,
    accounts: &[account::Model],
    metrics_map: &HashMap<i32, &AccountMetricsDto>,
) {
    // Generic debt repricing detection: any Debt account with refinancing/refix keywords
    let repricing_debts: Vec<&account::Model> = accounts
        .iter()
        .filter(|a| {
            if a.account_kind != AccountKind::Debt {
                return false;
            }
            let desc = a.description.as_deref().unwrap_or("");
            let desc_lower = desc.to_lowercase();
            desc_lower.contains("fixation")
                || desc_lower.contains("refix")
                || desc_lower.contains("refinanc")
                || desc_lower.contains("interest rate change")
                || desc_lower.contains("strike")
                || desc_lower.contains("rate change")
                || desc_lower.contains("repricing")
        })
        .collect();

    if repricing_debts.is_empty() {
        return;
    }

    out.push_str("## DEBT REPRICING / REFINANCING RISK\n\n");
    out.push_str("_The following debt accounts have upcoming rate changes or refinancing events._\n\n");

    for a in &repricing_debts {
        let _ = writeln!(out, "### {}\n", a.name);

        if let Some(desc) = &a.description {
            let _ = writeln!(out, "- Description: {}", desc);
        }

        if let Some(m) = metrics_map.get(&a.id) {
            if let Some(AccountKindMetricsDto::Debt(ref debt)) = m.kind_metrics {
                if let Some(principal) = debt.outstanding_principal {
                    let _ = writeln!(
                        out,
                        "- Outstanding Principal: {}",
                        principal.round_dp(0)
                    );
                }
                if let Some(payment) = debt.required_monthly_payment {
                    let _ = writeln!(
                        out,
                        "- Current Monthly Payment: {}",
                        payment.round_dp(0)
                    );
                }
                if let Some(date) = debt.debt_free_date {
                    let _ = writeln!(out, "- Debt-Free Date: {}", date);
                }
            }
        }

        out.push_str("\n**Rate Change Scenario Estimates** _(approximate, for LLM analysis)_:\n");
        out.push_str("- The LLM should estimate payment changes at various interest rates (e.g., +1%, +2%, +3%)\n");
        out.push_str("  based on the outstanding principal and remaining term from the data above.\n");
        out.push_str("- Check if any strike/prepayment fund or reserve exists and what impact it would have.\n");
        out.push_str("- Determine whether this repricing is imminent or future-dated.\n\n");
    }
}

// ---------------------------------------------------------------------------
// Goal engine split
// ---------------------------------------------------------------------------

fn write_goal_engine_split(
    out: &mut String,
    accounts: &[account::Model],
    account_metrics: &[AccountMetricsDto],
) {
    out.push_str("## GOAL ENGINE BREAKDOWN\n\n");
    out.push_str("_Splits monthly inflows by purpose: safety vs consumption vs wealth._\n\n");
    out.push_str("_Uses DERIVED account roles (not just base types) to classify flows correctly._\n\n");

    let mut safety_total = Decimal::ZERO;
    let mut consumption_total = Decimal::ZERO;
    let mut wealth_total = Decimal::ZERO;

    let mut safety_items: Vec<(String, Decimal)> = Vec::new();
    let mut consumption_items: Vec<(String, Decimal)> = Vec::new();
    let mut wealth_items: Vec<(String, Decimal)> = Vec::new();

    for a in accounts {
        let role = compute_account_role(a);
        let net_flow = account_metrics
            .iter()
            .find(|m| m.account_id == a.id)
            .and_then(|m| m.monthly_net_flow)
            .unwrap_or(Decimal::ZERO);

        if net_flow <= Decimal::ZERO {
            continue;
        }

        // Skip operating accounts — their inflows are income, not goal contributions
        if role.role == "operating" {
            continue;
        }

        match role.role.as_str() {
            "emergency_reserve" | "reserved_liability" | "income_smoothing"
            | "maintenance_reserve" => {
                safety_total += net_flow;
                safety_items.push((a.name.clone(), net_flow));
            }
            "sinking_fund" | "personal_allowance" | "family_discretionary" | "savings" => {
                consumption_total += net_flow;
                consumption_items.push((a.name.clone(), net_flow));
            }
            "investment" | "equity_investment" | "retirement" => {
                wealth_total += net_flow;
                wealth_items.push((a.name.clone(), net_flow));
            }
            _ => {
                consumption_total += net_flow;
                consumption_items.push((a.name.clone(), net_flow));
            }
        }
    }

    out.push_str("| Engine | Monthly Total | Accounts |\n");
    out.push_str("|--------|---------------|----------|\n");

    let fmt_items = |items: &[(String, Decimal)]| -> String {
        if items.is_empty() {
            return "none".to_string();
        }
        items
            .iter()
            .map(|(name, flow)| format!("{} ({})", name, flow.round_dp(0)))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let _ = writeln!(
        out,
        "| **Safety** (emergency + tax + income smoothing + maintenance) | {} | {} |",
        safety_total.round_dp(0),
        fmt_items(&safety_items)
    );
    let _ = writeln!(
        out,
        "| **Consumption goals** (vacation, sinking funds, allowances) | {} | {} |",
        consumption_total.round_dp(0),
        fmt_items(&consumption_items)
    );
    let _ = writeln!(
        out,
        "| **Wealth building** (investments, retirement, equity) | {} | {} |",
        wealth_total.round_dp(0),
        fmt_items(&wealth_items)
    );
    out.push_str("\n_WARNING: Do not count safety and consumption goals as \"savings\" or wealth building._\n\n");
}

// ---------------------------------------------------------------------------
// Recent financial changes
// ---------------------------------------------------------------------------

async fn write_recent_financial_changes(
    out: &mut String,
    db: &DatabaseConnection,
    accounts: &[account::Model],
    today: NaiveDate,
) {
    out.push_str("## RECENT FINANCIAL CHANGES\n\n");
    out.push_str("_Transactions that started or ended recently (last 90 days). ");
    out.push_str("This is critical context: the LLM must know when systems changed recently ");
    out.push_str("and forecast strength is not yet proven._\n\n");

    let ninety_days_ago = today
        .checked_sub_signed(chrono::Duration::days(90))
        .unwrap_or(today);

    let account_map: HashMap<i32, &account::Model> =
        accounts.iter().map(|a| (a.id, a)).collect();

    // Recently started recurring transactions
    let recently_started: Vec<recurring_transaction::Model> =
        match recurring_transaction::Entity::find()
            .filter(recurring_transaction::Column::IsSimulated.eq(false))
            .filter(recurring_transaction::Column::StartDate.between(ninety_days_ago, today))
            .all(db)
            .await
        {
            Ok(t) => t,
            Err(_) => Vec::new(),
        };

    // Recently ended recurring transactions
    let recently_ended: Vec<recurring_transaction::Model> =
        match recurring_transaction::Entity::find()
            .filter(recurring_transaction::Column::IsSimulated.eq(false))
            .filter(recurring_transaction::Column::EndDate.between(ninety_days_ago, today))
            .all(db)
            .await
        {
            Ok(t) => t,
            Err(_) => Vec::new(),
        };

    // Recently started recurring incomes
    let recently_started_income: Vec<recurring_income::Model> =
        match recurring_income::Entity::find()
            .filter(recurring_income::Column::IsSimulated.eq(false))
            .filter(recurring_income::Column::StartDate.between(ninety_days_ago, today))
            .all(db)
            .await
        {
            Ok(t) => t,
            Err(_) => Vec::new(),
        };

    // Recently ended recurring incomes
    let recently_ended_income: Vec<recurring_income::Model> =
        match recurring_income::Entity::find()
            .filter(recurring_income::Column::IsSimulated.eq(false))
            .filter(recurring_income::Column::EndDate.between(ninety_days_ago, today))
            .all(db)
            .await
        {
            Ok(t) => t,
            Err(_) => Vec::new(),
        };

    if recently_started.is_empty()
        && recently_ended.is_empty()
        && recently_started_income.is_empty()
        && recently_ended_income.is_empty()
    {
        out.push_str("No recent changes detected in the last 90 days.\n\n");
        return;
    }

    out.push_str("| Change | Type | Amount | Period | Account | Confidence |\n");
    out.push_str("|--------|------|--------|--------|---------|------------|\n");

    for t in &recently_started {
        let acct = account_map
            .get(&t.target_account_id)
            .map(|a| a.name.as_str())
            .unwrap_or("-");
        let _ = writeln!(
            out,
            "| NEW: {} | expense/transfer | {} | {} | {} | low (newly introduced) |",
            t.name,
            t.amount.round_dp(0),
            period_str(&t.period),
            acct,
        );
    }

    for t in &recently_ended {
        let acct = account_map
            .get(&t.target_account_id)
            .map(|a| a.name.as_str())
            .unwrap_or("-");
        let _ = writeln!(
            out,
            "| ENDED: {} | expense/transfer | {} | {} | {} | medium (may be temporary pause) |",
            t.name,
            t.amount.round_dp(0),
            period_str(&t.period),
            acct,
        );
    }

    for t in &recently_started_income {
        let acct = account_map
            .get(&t.target_account_id)
            .map(|a| a.name.as_str())
            .unwrap_or("-");
        let _ = writeln!(
            out,
            "| NEW INCOME: {} | income | {} | {} | {} | low (newly introduced) |",
            t.name,
            t.amount.round_dp(0),
            period_str(&t.period),
            acct,
        );
    }

    for t in &recently_ended_income {
        let acct = account_map
            .get(&t.target_account_id)
            .map(|a| a.name.as_str())
            .unwrap_or("-");
        let _ = writeln!(
            out,
            "| ENDED INCOME: {} | income | {} | {} | {} | high (confirmed ended) |",
            t.name,
            t.amount.round_dp(0),
            period_str(&t.period),
            acct,
        );
    }

    out.push_str("\n_Changes marked 'low' confidence mean the forecast depends on new behavior not yet proven historically._\n\n");
}

// ---------------------------------------------------------------------------
// Known future events
// ---------------------------------------------------------------------------

fn write_known_future_events(out: &mut String, accounts: &[account::Model]) {
    out.push_str("## KNOWN FUTURE EVENTS\n\n");
    out.push_str("_Extracted from account descriptions using generic keyword detection._\n\n");

    let mut events: Vec<String> = Vec::new();

    for a in accounts {
        let desc = a.description.as_deref().unwrap_or("");
        let desc_lower = desc.to_lowercase();

        // Generic debt repricing detection
        if a.account_kind == AccountKind::Debt
            && (desc_lower.contains("fixation")
                || desc_lower.contains("refix")
                || desc_lower.contains("refinanc")
                || desc_lower.contains("interest rate change")
                || desc_lower.contains("strike")
                || desc_lower.contains("rate change")
                || desc_lower.contains("repricing"))
        {
            events.push(format!(
                "- **Debt repricing / rate change** ({}): {}",
                a.name, desc
            ));
        }

        // Generic purchase / downpayment detection
        if desc_lower.contains("down payment")
            || desc_lower.contains("downpayment")
            || desc_lower.contains("purchase")
            || desc_lower.contains("buy")
        {
            events.push(format!(
                "- **Planned purchase** ({}): {}",
                a.name, desc
            ));
        }

        // Generic loan payoff targets
        if a.account_kind == AccountKind::Debt
            && (desc_lower.contains("get rid")
                || desc_lower.contains("pay off")
                || desc_lower.contains("goal"))
        {
            events.push(format!(
                "- **Debt payoff goal** ({}): {}",
                a.name, desc
            ));
        }

        // Income seasonality / variable income
        if desc_lower.contains("seasonal")
            || desc_lower.contains("variable income")
            || desc_lower.contains("low-income month")
            || (desc_lower.contains("osvc") && desc_lower.contains("buffer"))
        {
            events.push(format!(
                "- **Income variability** ({}): {}",
                a.name, desc
            ));
        }

        // Target date / goal date detection
        if a.target_amount.is_some()
            && (desc_lower.contains("by ")
                || desc_lower.contains("target date")
                || desc_lower.contains("goal date"))
        {
            events.push(format!(
                "- **Target date goal** ({}): {}",
                a.name, desc
            ));
        }
    }

    if events.is_empty() {
        out.push_str("No specific future events detected from account descriptions.\n");
        out.push_str("_The LLM should ask the user about upcoming large expenses, income changes, ");
        out.push_str("or life events that would affect the financial plan._\n\n");
    } else {
        for event in &events {
            let _ = writeln!(out, "{}", event);
        }
        out.push_str("\n_The LLM must incorporate these events into its forecast challenge and action recommendations._\n\n");
    }
}

// ---------------------------------------------------------------------------
// User prompt
// ---------------------------------------------------------------------------

fn write_user_prompt(out: &mut String) {
    out.push_str(
        r#"<user>
Based on all the financial data above, assess my financial situation — but be skeptical.
Explicitly identify what looks good only on paper versus what is actually safe and proven.

START WITH A MANDATORY "BOTTOM LINE" SECTION that directly answers:
- Are finances currently safe or fragile?
- Is the household genuinely improving, or just forecasted to improve?
- What is the single biggest problem right now?
- What is the single most important next fix?

Then provide the full analysis using the required 4-layer evaluation structure:

1. **Liquidity / Safety assessment**
   - What are my real liquid reserves (excluding tax reserves, sinking funds, earmarked money)?
   - How safe am I against a 1-month, 3-month, and 6-month income disruption?
   - Is my emergency fund adequate or just started?

2. **Cashflow quality analysis**
   - What is my operating free cashflow after removing internal transfers and allocations?
   - What is being counted as "savings" that is really future spending or internal routing?
   - Break down: mandatory burn vs controllable burn vs discretionary burn.

3. **Behavioral leakage / spending control**
   - What historical spending leaks are the biggest threat to this plan?
   - Which current improvements are structural vs only temporary?
   - Are the control systems (allowance accounts, shared account caps) proven or newly introduced?
   - What am I likely underestimating?

4. **Long-term wealth building**
   - Is money actually going to real investments and retirement?
   - Are debt paydowns on track? What about the mortgage refix risk?
   - Am I genuinely building wealth or just reshuffling money between accounts?

Then answer these additional questions:

5. **Forecast challenge** — Which forecast assumptions are fragile or overly optimistic?
   Separate mechanically projected outcomes from behaviorally credible ones.

6. **Account organization** — Is each account serving a clear, distinct purpose?
   Are any account types misleading (e.g., maintenance reserve labeled as emergency fund)?

7. **Goal review** — For accounts with targets, am I on track? Are the targets realistic
   given current cashflow quality (not just forecast)?

8. **12 / 24 / 36 month goals** — Concrete, measurable targets. Be realistic, not optimistic.

9. **Priority actions** — Use the Stop / Pause / Reduce / Keep / Increase / Create format.
   For each action: why it matters, monthly impact, and what it improves
   (liquidity / discipline / long-term growth / debt reduction).

10. **Risk assessment** — What are the biggest financial risks? Include:
    - Mortgage refix impact scenarios
    - Known upcoming large expenses (car, etc.)
    - What happens if the spending control system fails?
    - What should I stop, pause, reduce, keep, increase, or create right now?
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
        AccountKind::Shared => "SharedDiscretionary (Family Discretionary)",
        AccountKind::EmergencyFund => "Emergency Fund",
        AccountKind::Equity => "Equity",
        AccountKind::House => "House",
        AccountKind::Tax => "TaxReserve (Reserved Liability)",
    }
}

/// Derives account role metadata purely from the `AccountKind` enum.
///
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
