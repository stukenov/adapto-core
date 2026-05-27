use crate::event::{AuditEvent, AuditStatus};
use chrono::{DateTime, Utc};

pub struct AuditFilter {
    pub event_name: Option<String>,
    pub action: Option<String>,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
    pub status: Option<StatusFilter>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub route_prefix: Option<String>,
}

pub enum StatusFilter {
    Success,
    Failure,
    Denied,
}

impl AuditFilter {
    pub fn new() -> Self {
        Self {
            event_name: None,
            action: None,
            user_id: None,
            tenant_id: None,
            status: None,
            from: None,
            to: None,
            route_prefix: None,
        }
    }

    pub fn event(mut self, name: &str) -> Self {
        self.event_name = Some(name.into());
        self
    }

    pub fn action(mut self, action: &str) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn status(mut self, status: StatusFilter) -> Self {
        self.status = Some(status);
        self
    }

    pub fn from(mut self, from: DateTime<Utc>) -> Self {
        self.from = Some(from);
        self
    }

    pub fn to(mut self, to: DateTime<Utc>) -> Self {
        self.to = Some(to);
        self
    }

    pub fn route_prefix(mut self, prefix: &str) -> Self {
        self.route_prefix = Some(prefix.into());
        self
    }

    pub fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(ref name) = self.event_name {
            if event.event != *name {
                return false;
            }
        }
        if let Some(ref action) = self.action {
            if event.action != *action {
                return false;
            }
        }
        if let Some(ref uid) = self.user_id {
            match &event.user_id {
                Some(u) => {
                    if u.0.to_string() != *uid {
                        return false;
                    }
                }
                None => return false,
            }
        }
        if let Some(ref tid) = self.tenant_id {
            match &event.tenant_id {
                Some(t) => {
                    if t.0.to_string() != *tid {
                        return false;
                    }
                }
                None => return false,
            }
        }
        if let Some(ref sf) = self.status {
            let matches_status = match sf {
                StatusFilter::Success => event.status == AuditStatus::Success,
                StatusFilter::Failure => matches!(event.status, AuditStatus::Failure(_)),
                StatusFilter::Denied => event.status == AuditStatus::Denied,
            };
            if !matches_status {
                return false;
            }
        }
        if let Some(ref from) = self.from {
            if event.timestamp < *from {
                return false;
            }
        }
        if let Some(ref to) = self.to {
            if event.timestamp > *to {
                return false;
            }
        }
        if let Some(ref prefix) = self.route_prefix {
            if !event.route.starts_with(prefix.as_str()) {
                return false;
            }
        }
        true
    }
}

impl Default for AuditFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::sink::InMemoryAuditSink {
    pub fn query(&self, filter: &AuditFilter) -> Vec<AuditEvent> {
        self.events()
            .into_iter()
            .filter(|e| filter.matches(e))
            .collect()
    }

    pub fn count_matching(&self, filter: &AuditFilter) -> usize {
        self.events().iter().filter(|e| filter.matches(e)).count()
    }
}
