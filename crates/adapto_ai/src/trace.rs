use crate::action::{AiRequest, TokenUsage};
use adapto_runtime::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// A single trace record for one AI action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTrace {
    pub trace_id: String,
    pub action: String,
    pub model: String,
    pub tenant_id: Option<TenantId>,
    pub user_id: Option<UserId>,
    pub request_id: RequestId,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub latency_ms: Option<u64>,
    pub tokens: Option<TokenUsage>,
    pub status: TraceStatus,
    pub pii_redacted: bool,
    pub retries: u32,
    pub fallback_used: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TraceStatus {
    Started,
    Completed,
    Failed,
    TimedOut,
    BudgetExceeded,
}

/// Collects AI action traces for observability and debugging.
pub struct TraceCollector {
    traces: Arc<Mutex<Vec<AiTrace>>>,
}

impl TraceCollector {
    pub fn new() -> Self {
        Self {
            traces: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create and store a new trace in `Started` status.
    pub fn start_trace(&self, request: &AiRequest, model: &str) -> AiTrace {
        let trace = AiTrace {
            trace_id: uuid::Uuid::new_v4().to_string(),
            action: request.action.clone(),
            model: model.to_string(),
            tenant_id: request.tenant_id.clone(),
            user_id: request.user_id.clone(),
            request_id: request.request_id.clone(),
            started_at: Utc::now(),
            finished_at: None,
            latency_ms: None,
            tokens: None,
            status: TraceStatus::Started,
            pii_redacted: false,
            retries: 0,
            fallback_used: false,
            error: None,
        };

        let mut traces = self.traces.lock().unwrap();
        traces.push(trace.clone());
        trace
    }

    /// Store a completed (or failed) trace, replacing any existing
    /// trace with the same `trace_id`.
    pub fn complete_trace(&self, trace: AiTrace) {
        let mut traces = self.traces.lock().unwrap();
        if let Some(pos) = traces.iter().position(|t| t.trace_id == trace.trace_id) {
            traces[pos] = trace;
        } else {
            traces.push(trace);
        }
    }

    pub fn get_traces(&self) -> Vec<AiTrace> {
        let traces = self.traces.lock().unwrap();
        traces.clone()
    }

    pub fn get_traces_for_tenant(&self, tenant_id: &TenantId) -> Vec<AiTrace> {
        let traces = self.traces.lock().unwrap();
        traces
            .iter()
            .filter(|t| t.tenant_id.as_ref() == Some(tenant_id))
            .cloned()
            .collect()
    }
}

impl Default for TraceCollector {
    fn default() -> Self {
        Self::new()
    }
}
