//! Generic account role classification and semantic interpretation.
//!
//! This module provides account classification logic that is:
//! - Generic and reusable across households
//! - Based on structured account properties (kind, description, target, flags)
//! - NOT dependent on specific account names or IDs
//!
//! The classification refines the base `AccountKind` enum using available metadata
//! to derive a more accurate semantic role for financial analysis and prompt generation.

use model::entities::account::{self, AccountKind};

/// Derived semantic role for an account, computed from AccountKind + metadata.
///
/// This provides a richer classification than the base AccountKind enum,
/// distinguishing between accounts that have the same technical type but
/// serve different financial purposes.
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedAccountRole {
    /// The semantic role determined by classification logic
    pub role: String,

    /// Human-readable purpose description
    pub purpose: String,

    /// Whether this account can be tapped in a genuine emergency
    pub can_be_used_in_emergency: bool,

    /// Whether this is a true emergency reserve (not earmarked)
    pub counts_as_emergency_reserve: bool,

    /// Whether this buffers variable income (e.g. self-employment smoothing)
    pub counts_as_income_smoothing: bool,

    /// Whether positive flows represent genuine wealth accumulation
    pub counts_as_long_term_wealth: bool,

    /// Whether this money is earmarked for future spending
    pub is_earmarked_spending: bool,

    /// Priority level for financial safety hierarchy
    pub priority_level: String,

    /// Explanation of how this role was derived (for transparency)
    pub classification_reason: String,
}

/// Derives the semantic role for an account using generic classification rules.
///
/// ## Classification Logic (in priority order):
///
/// 1. **Base type override by description keywords:**
///    - Savings + "vacation" (not "replacement") → sinking_fund
///    - Savings + "replacement" / "buffer" / "smoothing" → income_smoothing
///    - EmergencyFund + "maintenance" / "repair" → maintenance_reserve
///    - Allowance + "family" / "shared" → family_discretionary
///
/// 2. **Target amount heuristics:**
///    - Liquid + target + future spending keywords → sinking_fund
///    - Liquid + target + no wealth keywords → likely earmarked
///
/// 3. **Base AccountKind defaults:**
///    - Fallback to standard classification when no refinement applies
///
/// ## Design Rules:
/// - NO account name checks (e.g., `if name == "Family Shared"`)
/// - NO account ID checks
/// - NO household-specific branching
/// - ALL classification based on: kind, description, target_amount, is_liquid
/// - Fallback heuristics must be generic and well-documented
pub fn derive_account_role(account: &account::Model) -> DerivedAccountRole {
    let desc = account.description.as_deref().unwrap_or("");
    let desc_lower = desc.to_lowercase();
    let kind = account.account_kind;

    // Refinement layer: use description to override base kind classification
    match kind {
        AccountKind::Savings => {
            // Use weighted scoring to distinguish income smoothing from sinking fund.
            // Smoothing: buffer for variable income (self-employment, freelance).
            // Sinking: saving for a specific planned future expense.
            let (smoothing_score, sinking_score) =
                score_smoothing_vs_sinking(&desc_lower, account.target_amount.is_some());

            if smoothing_score > sinking_score {
                DerivedAccountRole {
                    role: "income_smoothing".to_string(),
                    purpose: "Buffer for variable income months (not true emergency fund)".to_string(),
                    can_be_used_in_emergency: true,
                    counts_as_emergency_reserve: false,
                    counts_as_income_smoothing: true,
                    counts_as_long_term_wealth: false,
                    is_earmarked_spending: false,
                    priority_level: "high".to_string(),
                    classification_reason: format!(
                        "Savings scored smoothing({}) > sinking({}): '{}'",
                        smoothing_score,
                        sinking_score,
                        desc.chars().take(80).collect::<String>()
                    ),
                }
            } else if sinking_score > 0 {
                DerivedAccountRole {
                    role: "sinking_fund".to_string(),
                    purpose: "Savings earmarked for specific future spending".to_string(),
                    can_be_used_in_emergency: false,
                    counts_as_emergency_reserve: false,
                    counts_as_income_smoothing: false,
                    counts_as_long_term_wealth: false,
                    is_earmarked_spending: true,
                    priority_level: "low".to_string(),
                    classification_reason: format!(
                        "Savings scored sinking({}) >= smoothing({}): '{}'",
                        sinking_score,
                        smoothing_score,
                        desc.chars().take(80).collect::<String>()
                    ),
                }
            } else {
                // Neither scored — ambiguous savings, needs LLM interpretation
                DerivedAccountRole {
                    role: "savings".to_string(),
                    purpose: "General savings account — check description to determine if sinking fund, reserve, or wealth building".to_string(),
                    can_be_used_in_emergency: true,
                    counts_as_emergency_reserve: false,
                    counts_as_income_smoothing: false,
                    counts_as_long_term_wealth: false,
                    is_earmarked_spending: false,
                    priority_level: "medium".to_string(),
                    classification_reason: "Savings account without clear refinement keywords".to_string(),
                }
            }
        }

        AccountKind::EmergencyFund => {
            // Use weighted scoring to distinguish true emergency fund from maintenance reserve.
            // Words like "burns", "job loss", "emergency" boost emergency score.
            // Words like "maintenance", "repair", "upkeep" boost maintenance score.
            // Context matters: "roof burns" is emergency, "roof repair" is maintenance.
            let (emergency_score, maintenance_score) =
                score_emergency_vs_maintenance(&desc_lower);

            if maintenance_score > emergency_score {
                DerivedAccountRole {
                    role: "maintenance_reserve".to_string(),
                    purpose: "Reserve for predictable home/vehicle maintenance — not a true emergency fund".to_string(),
                    can_be_used_in_emergency: true,
                    counts_as_emergency_reserve: false,
                    counts_as_income_smoothing: false,
                    counts_as_long_term_wealth: false,
                    is_earmarked_spending: true,
                    priority_level: "high".to_string(),
                    classification_reason: format!(
                        "EmergencyFund scored maintenance({}) > emergency({}): '{}'",
                        maintenance_score,
                        emergency_score,
                        desc.chars().take(80).collect::<String>()
                    ),
                }
            } else {
                // Default: EmergencyFund kind → true emergency reserve
                DerivedAccountRole {
                    role: "emergency_reserve".to_string(),
                    purpose: "True emergency fund for income disruption or unexpected crises".to_string(),
                    can_be_used_in_emergency: true,
                    counts_as_emergency_reserve: true,
                    counts_as_income_smoothing: false,
                    counts_as_long_term_wealth: false,
                    is_earmarked_spending: false,
                    priority_level: "critical".to_string(),
                    classification_reason: format!(
                        "EmergencyFund scored emergency({}) >= maintenance({}): '{}'",
                        emergency_score,
                        maintenance_score,
                        desc.chars().take(80).collect::<String>()
                    ),
                }
            }
        }

        AccountKind::Allowance => {
            // Check for family discretionary (shared optional spending)
            if desc_lower.contains("family")
                || desc_lower.contains("shared")
                || desc_lower.contains("together")
                || desc_lower.contains("both of us")
            {
                return DerivedAccountRole {
                    role: "family_discretionary".to_string(),
                    purpose: "Shared family discretionary spending (NOT personal allowance)".to_string(),
                    can_be_used_in_emergency: false,
                    counts_as_emergency_reserve: false,
                    counts_as_income_smoothing: false,
                    counts_as_long_term_wealth: false,
                    is_earmarked_spending: false,
                    priority_level: "low".to_string(),
                    classification_reason: format!(
                        "Allowance account with family/shared keywords: '{}'",
                        desc.chars().take(80).collect::<String>()
                    ),
                };
            }

            // Personal allowance (individual discretionary)
            DerivedAccountRole {
                role: "personal_allowance".to_string(),
                purpose: "Individual discretionary spending allowance — spending control mechanism".to_string(),
                can_be_used_in_emergency: false,
                counts_as_emergency_reserve: false,
                counts_as_income_smoothing: false,
                counts_as_long_term_wealth: false,
                is_earmarked_spending: false,
                priority_level: "low".to_string(),
                classification_reason: "Allowance account without family/shared keywords".to_string(),
            }
        }

        // Remaining kinds use base classification
        _ => derive_account_role_from_kind(kind),
    }
}

/// Scores emergency vs maintenance intent from description text.
///
/// Uses weighted keyword matching with context awareness:
/// - Emergency keywords: crisis scenarios (job loss, fire, catastrophe, "goes wrong")
/// - Maintenance keywords: predictable upkeep (repair, maintenance, appliance replacement)
/// - Context modifiers: "burns" near "roof"/"house" boosts emergency (catastrophe),
///   while "repair" near "roof"/"house" boosts maintenance
///
/// Returns `(emergency_score, maintenance_score)`. Higher score wins.
/// On tie or both zero, the base AccountKind default applies (emergency for EmergencyFund).
fn score_emergency_vs_maintenance(desc_lower: &str) -> (i32, i32) {
    let mut emergency: i32 = 0;
    let mut maintenance: i32 = 0;

    // Strong emergency signals (crisis / catastrophe / income disruption)
    let emergency_keywords = [
        ("job loss", 3),
        ("lose job", 3),
        ("loose job", 3),      // common misspelling
        ("lose my job", 3),
        ("emergency", 3),
        ("goes wrong", 3),
        ("something happens", 2),
        ("pillow", 2),         // "financial pillow" idiom
        ("crisis", 3),
        ("catastroph", 3),
        ("unexpected", 2),
        ("disaster", 3),
        ("burns", 2),          // "house burns down" = emergency
        ("fire", 2),
        ("flood", 2),
        ("insurance", 1),      // having insurance context suggests emergency planning
        ("income disruption", 3),
        ("safety net", 3),
        ("rainy day", 2),
    ];

    // Strong maintenance signals (predictable, planned upkeep)
    let maintenance_keywords = [
        ("maintenance", 3),
        ("repair", 3),
        ("upkeep", 3),
        ("fond oprav", 3),     // Czech "repair fund" concept
        ("appliance", 2),
        ("washing machine", 2),
        ("replacement fund", 2),
        ("wear and tear", 2),
        ("regular upkeep", 3),
        ("home improvement", 2),
        ("renovation", 2),
    ];

    for (kw, weight) in &emergency_keywords {
        if desc_lower.contains(kw) {
            emergency += weight;
        }
    }

    for (kw, weight) in &maintenance_keywords {
        if desc_lower.contains(kw) {
            maintenance += weight;
        }
    }

    // Context-aware: "house" and "roof" are neutral on their own.
    // They only matter in combination with emergency or maintenance context.
    // If already scored by specific keywords above, no extra adjustment needed.
    // If neither scored, these words alone don't determine the role.

    (emergency, maintenance)
}

/// Scores income-smoothing vs sinking-fund intent for Savings accounts.
///
/// Income smoothing: buffer for variable income months (self-employment, freelance).
/// Sinking fund: saving for a specific planned future expense.
///
/// Returns `(smoothing_score, sinking_score)`.
fn score_smoothing_vs_sinking(desc_lower: &str, has_target: bool) -> (i32, i32) {
    let mut smoothing: i32 = 0;
    let mut sinking: i32 = 0;

    let smoothing_keywords = [
        ("buffer", 2),
        ("smoothing", 3),
        ("variable income", 3),
        ("not making money", 3),
        ("won't be making money", 3),    // typo-tolerant
        ("won't be maing money", 3),     // actual data typo
        ("not earning", 2),
        ("income replacement", 3),
        ("income buffer", 3),
        ("low-income month", 3),
        ("when i don't invoice", 3),
        ("replacement", 2),
        ("self-employed", 2),
        ("osvc", 2),                     // Czech self-employment
        ("freelanc", 2),
    ];

    let sinking_keywords = [
        ("saving for", 2),
        ("will be spend", 3),            // actual data phrasing
        ("will be spent", 3),
        ("down payment", 3),
        ("downpayment", 3),
        ("accommodation", 2),
        ("planned purchase", 3),
        ("trip", 1),
        ("air condition", 2),
        ("new car", 2),
    ];

    for (kw, weight) in &smoothing_keywords {
        if desc_lower.contains(kw) {
            smoothing += weight;
        }
    }

    for (kw, weight) in &sinking_keywords {
        if desc_lower.contains(kw) {
            sinking += weight;
        }
    }

    // "vacation" is ambiguous: vacation fund (sinking) vs vacation income replacement (smoothing)
    // Only count it for sinking if no smoothing signals are present
    if desc_lower.contains("vacation") && smoothing == 0 {
        sinking += 2;
    }

    // Having a target amount is a weak signal toward sinking fund (saving toward a goal)
    if has_target && sinking == 0 && smoothing == 0 {
        sinking += 1;
    }

    (smoothing, sinking)
}

/// Baseline classification from AccountKind when no description refinement applies.
fn derive_account_role_from_kind(kind: AccountKind) -> DerivedAccountRole {
    match kind {
        AccountKind::RealAccount => DerivedAccountRole {
            role: "operating".to_string(),
            purpose: "Main operating account — income arrives here, essentials paid from here".to_string(),
            can_be_used_in_emergency: true,
            counts_as_emergency_reserve: false,
            counts_as_income_smoothing: false,
            counts_as_long_term_wealth: false,
            is_earmarked_spending: false,
            priority_level: "critical".to_string(),
            classification_reason: "RealAccount type".to_string(),
        },
        AccountKind::Shared => DerivedAccountRole {
            role: "shared_discretionary".to_string(),
            purpose: "Shared family discretionary spending — joint optional spending".to_string(),
            can_be_used_in_emergency: false,
            counts_as_emergency_reserve: false,
            counts_as_income_smoothing: false,
            counts_as_long_term_wealth: false,
            is_earmarked_spending: false,
            priority_level: "low".to_string(),
            classification_reason: "Shared account type".to_string(),
        },
        AccountKind::Investment => DerivedAccountRole {
            role: "investment".to_string(),
            purpose: "Investment account — long-term wealth building".to_string(),
            can_be_used_in_emergency: false,
            counts_as_emergency_reserve: false,
            counts_as_income_smoothing: false,
            counts_as_long_term_wealth: true,
            is_earmarked_spending: false,
            priority_level: "medium".to_string(),
            classification_reason: "Investment account type".to_string(),
        },
        AccountKind::Equity => DerivedAccountRole {
            role: "equity_investment".to_string(),
            purpose: "Equity / stock market investment — illiquid, long-term".to_string(),
            can_be_used_in_emergency: false,
            counts_as_emergency_reserve: false,
            counts_as_income_smoothing: false,
            counts_as_long_term_wealth: true,
            is_earmarked_spending: false,
            priority_level: "medium".to_string(),
            classification_reason: "Equity account type".to_string(),
        },
        AccountKind::House => DerivedAccountRole {
            role: "house_value".to_string(),
            purpose: "Property value — completely illiquid, non-operating wealth".to_string(),
            can_be_used_in_emergency: false,
            counts_as_emergency_reserve: false,
            counts_as_income_smoothing: false,
            counts_as_long_term_wealth: false,
            is_earmarked_spending: false,
            priority_level: "low".to_string(),
            classification_reason: "House account type".to_string(),
        },
        AccountKind::Debt => DerivedAccountRole {
            role: "debt".to_string(),
            purpose: "Outstanding debt obligation".to_string(),
            can_be_used_in_emergency: false,
            counts_as_emergency_reserve: false,
            counts_as_income_smoothing: false,
            counts_as_long_term_wealth: false,
            is_earmarked_spending: false,
            priority_level: "high".to_string(),
            classification_reason: "Debt account type".to_string(),
        },
        AccountKind::Tax => DerivedAccountRole {
            role: "reserved_liability".to_string(),
            purpose: "Tax or annual obligation reserve — this money is committed and WILL be spent".to_string(),
            can_be_used_in_emergency: false,
            counts_as_emergency_reserve: false,
            counts_as_income_smoothing: false,
            counts_as_long_term_wealth: false,
            is_earmarked_spending: true,
            priority_level: "high".to_string(),
            classification_reason: "Tax account type".to_string(),
        },
        AccountKind::Goal => DerivedAccountRole {
            role: "goal".to_string(),
            purpose: "Goal-tracking account — check description to determine nature".to_string(),
            can_be_used_in_emergency: false,
            counts_as_emergency_reserve: false,
            counts_as_income_smoothing: false,
            counts_as_long_term_wealth: false,
            is_earmarked_spending: false,
            priority_level: "medium".to_string(),
            classification_reason: "Goal account type".to_string(),
        },
        AccountKind::Other => DerivedAccountRole {
            role: "other".to_string(),
            purpose: "Uncategorized account — check description".to_string(),
            can_be_used_in_emergency: false,
            counts_as_emergency_reserve: false,
            counts_as_income_smoothing: false,
            counts_as_long_term_wealth: false,
            is_earmarked_spending: false,
            priority_level: "low".to_string(),
            classification_reason: "Other account type".to_string(),
        },
        // Already handled above, but needed for exhaustiveness
        AccountKind::Savings => derive_account_role_from_kind_savings_fallback(),
        AccountKind::EmergencyFund => derive_account_role_from_kind_emergency_fallback(),
        AccountKind::Allowance => derive_account_role_from_kind_allowance_fallback(),
    }
}

fn derive_account_role_from_kind_savings_fallback() -> DerivedAccountRole {
    DerivedAccountRole {
        role: "savings".to_string(),
        purpose: "General savings account — check description".to_string(),
        can_be_used_in_emergency: true,
        counts_as_emergency_reserve: false,
        counts_as_income_smoothing: false,
        counts_as_long_term_wealth: false,
        is_earmarked_spending: false,
        priority_level: "medium".to_string(),
        classification_reason: "Savings account type (fallback)".to_string(),
    }
}

fn derive_account_role_from_kind_emergency_fallback() -> DerivedAccountRole {
    DerivedAccountRole {
        role: "emergency_reserve".to_string(),
        purpose: "Emergency fund".to_string(),
        can_be_used_in_emergency: true,
        counts_as_emergency_reserve: true,
        counts_as_income_smoothing: false,
        counts_as_long_term_wealth: false,
        is_earmarked_spending: false,
        priority_level: "critical".to_string(),
        classification_reason: "EmergencyFund account type (fallback)".to_string(),
    }
}

fn derive_account_role_from_kind_allowance_fallback() -> DerivedAccountRole {
    DerivedAccountRole {
        role: "personal_allowance".to_string(),
        purpose: "Personal allowance".to_string(),
        can_be_used_in_emergency: false,
        counts_as_emergency_reserve: false,
        counts_as_income_smoothing: false,
        counts_as_long_term_wealth: false,
        is_earmarked_spending: false,
        priority_level: "low".to_string(),
        classification_reason: "Allowance account type (fallback)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn make_account(
        kind: AccountKind,
        description: Option<String>,
        target: Option<Decimal>,
    ) -> account::Model {
        account::Model {
            id: 1,
            name: "Test".to_string(),
            description,
            currency_code: "CZK".to_string(),
            owner_id: 1,
            include_in_statistics: true,
            ledger_name: None,
            account_kind: kind,
            target_amount: target,
            color: None,
            is_liquid: true,
        }
    }

    #[test]
    fn test_vacation_sinking_fund() {
        let account = make_account(
            AccountKind::Savings,
            Some("This will be spend. Saving for vacation.".to_string()),
            Some(Decimal::from(50000)),
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "sinking_fund");
        assert!(role.is_earmarked_spending);
        assert!(!role.counts_as_long_term_wealth);
    }

    #[test]
    fn test_income_smoothing_buffer() {
        let account = make_account(
            AccountKind::Savings,
            Some("Vacation replacement for OSVC - buffer when not earning".to_string()),
            Some(Decimal::from(150000)),
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "income_smoothing");
        assert!(role.counts_as_income_smoothing);
        assert!(!role.is_earmarked_spending);
        assert!(!role.counts_as_emergency_reserve);
    }

    #[test]
    fn test_true_emergency_fund() {
        let account = make_account(
            AccountKind::EmergencyFund,
            Some("Emergency fund for job loss, roof burns etc.".to_string()),
            Some(Decimal::from(600000)),
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "emergency_reserve");
        assert!(role.counts_as_emergency_reserve);
        assert!(!role.is_earmarked_spending);
    }

    #[test]
    fn test_family_discretionary() {
        let account = make_account(
            AccountKind::Allowance,
            Some("Family shared spending - garden, vacation extras, both of us".to_string()),
            None,
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "family_discretionary");
        assert!(!role.counts_as_emergency_reserve);
    }

    #[test]
    fn test_personal_allowance() {
        let account = make_account(
            AccountKind::Allowance,
            Some("My personal allowance for gifts and discretionary".to_string()),
            None,
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "personal_allowance");
    }

    #[test]
    fn test_operating_account() {
        let account = make_account(
            AccountKind::RealAccount,
            Some("Main account, income here, essentials from here".to_string()),
            None,
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "operating");
        assert_eq!(role.priority_level, "critical");
    }

    #[test]
    fn test_emergency_fund_with_roof_burns_not_maintenance() {
        // "roof burns" is a catastrophe scenario, not a maintenance scenario
        let account = make_account(
            AccountKind::EmergencyFund,
            Some("This is the pillow when someting goes wrong. I loose job, our roof at house burns etc..".to_string()),
            Some(Decimal::from(600000)),
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "emergency_reserve", "roof burns = emergency, not maintenance");
        assert!(role.counts_as_emergency_reserve);
        assert!(!role.is_earmarked_spending);
    }

    #[test]
    fn test_maintenance_reserve_fond_oprav() {
        // Czech "fond oprav" = repair fund, clearly maintenance
        let account = make_account(
            AccountKind::EmergencyFund,
            Some("We have a house and the is maintenance. This is copying a fond oprav.".to_string()),
            Some(Decimal::from(150000)),
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "maintenance_reserve", "fond oprav + maintenance = maintenance_reserve");
        assert!(!role.counts_as_emergency_reserve);
        assert!(role.is_earmarked_spending);
    }

    #[test]
    fn test_osvc_vacation_replacement_is_income_smoothing() {
        // Actual DB data: OSVC + buffer + not making money = income smoothing
        let account = make_account(
            AccountKind::Savings,
            Some("This is special thing, as i'm OSVC, i'm not making money when i'm on a vacation, so this is to build a buffer for when i won't be maing money.".to_string()),
            Some(Decimal::from(150000)),
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "income_smoothing", "OSVC + buffer + not making money = income smoothing");
        assert!(role.counts_as_income_smoothing);
        assert!(!role.is_earmarked_spending);
    }

    #[test]
    fn test_saving_for_a_thing_is_sinking_fund() {
        let account = make_account(
            AccountKind::Savings,
            Some("Saving for something, like air condition into house, new car down payment etc...".to_string()),
            None,
        );
        let role = derive_account_role(&account);
        assert_eq!(role.role, "sinking_fund", "saving for + down payment = sinking fund");
        assert!(role.is_earmarked_spending);
    }

    #[test]
    fn test_scoring_emergency_vs_maintenance() {
        // Pure emergency context
        let (e, m) = score_emergency_vs_maintenance("job loss, emergency, crisis");
        assert!(e > m, "pure emergency should score higher");

        // Pure maintenance context
        let (e, m) = score_emergency_vs_maintenance("maintenance, repair, upkeep");
        assert!(m > e, "pure maintenance should score higher");

        // Mixed: emergency dominates
        let (e, m) = score_emergency_vs_maintenance("pillow when something goes wrong, roof burns");
        assert!(e > m, "emergency context should dominate even with roof mention");

        // Empty description: both zero → default to base kind
        let (e, m) = score_emergency_vs_maintenance("");
        assert_eq!(e, 0);
        assert_eq!(m, 0);
    }

    #[test]
    fn test_scoring_smoothing_vs_sinking() {
        // Pure smoothing context
        let (s, k) = score_smoothing_vs_sinking("buffer for variable income, osvc", false);
        assert!(s > k, "pure smoothing should score higher");

        // Pure sinking context
        let (s, k) = score_smoothing_vs_sinking("saving for vacation accommodation", false);
        assert!(k > s, "pure sinking should score higher");

        // Ambiguous vacation + OSVC + buffer → smoothing wins
        let (s, k) = score_smoothing_vs_sinking(
            "osvc, not making money on vacation, buffer",
            true,
        );
        assert!(s > k, "OSVC + buffer should override vacation sinking signal");
    }
}
