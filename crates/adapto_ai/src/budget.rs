use adapto_runtime::types::TenantId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Tracks per-tenant AI token and cost budgets.
#[derive(Debug, Clone)]
pub struct BudgetTracker {
    budgets: Arc<RwLock<HashMap<TenantId, TenantBudget>>>,
}

/// A single tenant's budget allocation and usage.
#[derive(Debug, Clone)]
pub struct TenantBudget {
    pub total_tokens: u64,
    pub used_tokens: u64,
    pub total_cost: f64,
    pub used_cost: f64,
    pub max_tokens_per_request: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    #[error("Token budget exceeded for tenant: used {used}/{total}")]
    TokenBudgetExceeded { used: u64, total: u64 },

    #[error("Cost budget exceeded for tenant: used ${used:.4}/${total:.4}")]
    CostBudgetExceeded { used: f64, total: f64 },

    #[error("No budget configured for tenant")]
    NoBudget,
}

impl BudgetTracker {
    pub fn new() -> Self {
        Self {
            budgets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_budget(&self, tenant_id: &TenantId, budget: TenantBudget) {
        let mut budgets = self.budgets.write().unwrap();
        budgets.insert(tenant_id.clone(), budget);
    }

    /// Check whether a tenant can afford `estimated_tokens` more tokens.
    pub fn check_budget(
        &self,
        tenant_id: &TenantId,
        estimated_tokens: u64,
    ) -> Result<(), BudgetError> {
        let budgets = self.budgets.read().unwrap();
        let budget = budgets.get(tenant_id).ok_or(BudgetError::NoBudget)?;

        if budget.used_tokens + estimated_tokens > budget.total_tokens {
            return Err(BudgetError::TokenBudgetExceeded {
                used: budget.used_tokens,
                total: budget.total_tokens,
            });
        }

        if budget.used_cost >= budget.total_cost {
            return Err(BudgetError::CostBudgetExceeded {
                used: budget.used_cost,
                total: budget.total_cost,
            });
        }

        Ok(())
    }

    /// Record tokens and cost consumed by a completed request.
    pub fn record_usage(&self, tenant_id: &TenantId, tokens: u64, cost: f64) {
        let mut budgets = self.budgets.write().unwrap();
        if let Some(budget) = budgets.get_mut(tenant_id) {
            budget.used_tokens += tokens;
            budget.used_cost += cost;
        }
    }

    pub fn get_usage(&self, tenant_id: &TenantId) -> Option<TenantBudget> {
        let budgets = self.budgets.read().unwrap();
        budgets.get(tenant_id).cloned()
    }

    /// Reset a tenant's usage counters to zero without changing limits.
    pub fn reset(&self, tenant_id: &TenantId) {
        let mut budgets = self.budgets.write().unwrap();
        if let Some(budget) = budgets.get_mut(tenant_id) {
            budget.used_tokens = 0;
            budget.used_cost = 0.0;
        }
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}
