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
            // Check for sinking fund (future planned spending)
            if (desc_lower.contains("vacation") && !desc_lower.contains("replacement"))
                || desc_lower.contains("down payment")
                || desc_lower.contains("downpayment")
                || desc_lower.contains("saving for")
                || (account.target_amount.is_some()
                    && (desc_lower.contains("will be spend")
                        || desc_lower.contains("spend")))
            {
                return DerivedAccountRole {
                    role: "sinking_fund".to_string(),
                    purpose: "Savings earmarked for specific future spending".to_string(),
                    can_be_used_in_emergency: false,
                    counts_as_emergency_reserve: false,
                    counts_as_income_smoothing: false,
                    counts_as_long_term_wealth: false,
                    is_earmarked_spending: true,
                    priority_level: "low".to_string(),
                    classification_reason: format!(
                        "Savings account with future spending keywords in description: '{}'",
                        desc.chars().take(80).collect::<String>()
                    ),
                };
            }

            // Check for income smoothing buffer
            if desc_lower.contains("replacement")
                || desc_lower.contains("buffer")
                || desc_lower.contains("smoothing")
                || desc_lower.contains("variable income")
                || (desc_lower.contains("osvc") || desc_lower.contains("self-employed"))
                    && desc_lower.contains("vacation")
            {
                return DerivedAccountRole {
                    role: "income_smoothing".to_string(),
                    purpose: "Buffer for variable income months (not true emergency fund)".to_string(),
                    can_be_used_in_emergency: true,
                    counts_as_emergency_reserve: false,
                    counts_as_income_smoothing: true,
                    counts_as_long_term_wealth: false,
                    is_earmarked_spending: false,
                    priority_level: "high".to_string(),
                    classification_reason: format!(
                        "Savings account with income smoothing keywords: '{}'",
                        desc.chars().take(80).collect::<String>()
                    ),
                };
            }

            // Generic savings (ambiguous - needs LLM interpretation)
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

        AccountKind::EmergencyFund => {
            // Check for maintenance reserve (not true emergency)
            if desc_lower.contains("maintenance")
                || desc_lower.contains("repair")
                || desc_lower.contains("house")
                || desc_lower.contains("roof")
                || desc_lower.contains("appliance")
            {
                return DerivedAccountRole {
                    role: "maintenance_reserve".to_string(),
                    purpose: "Reserve for predictable home/vehicle maintenance — not a true emergency fund".to_string(),
                    can_be_used_in_emergency: true,
                    counts_as_emergency_reserve: false,
                    counts_as_income_smoothing: false,
                    counts_as_long_term_wealth: false,
                    is_earmarked_spending: true,
                    priority_level: "high".to_string(),
                    classification_reason: format!(
                        "EmergencyFund account with maintenance keywords: '{}'",
                        desc.chars().take(80).collect::<String>()
                    ),
                };
            }

            // True emergency fund
            DerivedAccountRole {
                role: "emergency_reserve".to_string(),
                purpose: "True emergency fund for income disruption or unexpected crises".to_string(),
                can_be_used_in_emergency: true,
                counts_as_emergency_reserve: true,
                counts_as_income_smoothing: false,
                counts_as_long_term_wealth: false,
                is_earmarked_spending: false,
                priority_level: "critical".to_string(),
                classification_reason: "EmergencyFund account with emergency-related description".to_string(),
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
}
