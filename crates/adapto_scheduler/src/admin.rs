//! Admin page rendering for the scheduler. Pure data → HTML (no web-framework
//! types), so it is testable and reusable by any consumer.

use crate::state::JobStatus;

/// Render the jobs table as a standalone HTML fragment.
///
/// `post_base` is the path the run-now buttons POST to; e.g. `"/admin/jobs"`
/// produces form actions `"/admin/jobs/<job>/run"`.
pub fn render(statuses: &[JobStatus], post_base: &str) -> String {
    let mut rows = String::new();
    for s in statuses {
        let last_run = s
            .last_run
            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|| "—".into());
        let next_run = s
            .next_run
            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|| "—".into());
        let dur = s
            .last_duration_ms
            .map(|m| format!("{m} ms"))
            .unwrap_or_else(|| "—".into());
        let err = s.last_error.as_deref().map(esc).unwrap_or_default();
        let err_row = if s.status == "failed" && !err.is_empty() {
            format!("<div class=\"err\">{err}</div>")
        } else {
            String::new()
        };
        rows.push_str(&format!(
            "<tr class=\"st-{status}\">\
             <td>{job}{err_row}</td><td>{lane}</td><td>{prio}</td>\
             <td><span class=\"badge\">{status}</span></td>\
             <td>{last_run}</td><td>{next_run}</td><td>{dur}</td>\
             <td><form method=\"post\" action=\"{base}/{job}/run\"><button>Run now</button></form></td>\
             </tr>",
            status = esc(&s.status),
            job = esc(&s.job),
            lane = esc(&s.lane),
            prio = esc(&s.priority),
            last_run = last_run,
            next_run = next_run,
            dur = dur,
            base = esc(post_base),
            err_row = err_row,
        ));
    }
    format!(
        "<h1>Scheduled Jobs</h1>\
         <table class=\"jobs\"><thead><tr>\
         <th>Job</th><th>Lane</th><th>Priority</th><th>Status</th>\
         <th>Last run</th><th>Next run</th><th>Duration</th><th></th>\
         </tr></thead><tbody>{rows}</tbody></table>\
         <style>.jobs{{width:100%;border-collapse:collapse;font:14px -apple-system,sans-serif}}\
         .jobs th,.jobs td{{text-align:left;padding:8px 10px;border-bottom:1px solid #eee}}\
         .jobs th{{font-size:12px;color:#888;text-transform:uppercase;letter-spacing:.3px}}\
         .st-failed{{background:#fff5f5}} .st-running{{background:#f0f7ff}}\
         .badge{{font-size:12px;padding:2px 8px;border-radius:10px;background:#eee}}\
         .st-ok .badge{{background:#e6f4ea;color:#137333}}\
         .st-failed .badge{{background:#fce8e6;color:#c5221f}}\
         .st-running .badge{{background:#e8f0fe;color:#1a73e8}}\
         .err{{color:#c0392b;font-size:12px;margin-top:4px}}\
         .jobs button{{padding:4px 10px;cursor:pointer}}</style>"
    )
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::JobStatus;

    fn st(job: &str, status: &str) -> JobStatus {
        JobStatus {
            job: job.into(),
            lane: "light".into(),
            priority: "high".into(),
            schedule: "DailyAt 06:30".into(),
            status: status.into(),
            last_run: None,
            last_duration_ms: Some(412),
            next_run: None,
            last_error: if status == "failed" {
                Some("<boom>".into())
            } else {
                None
            },
            consecutive_failures: 0,
            total_runs: 1,
            total_failures: 0,
        }
    }

    #[test]
    fn renders_rows_and_run_button() {
        let html = render(&[st("currency", "ok")], "/admin/jobs");
        assert!(html.contains("currency"));
        assert!(html.contains("action=\"/admin/jobs/currency/run\""));
        assert!(html.contains("Run now"));
    }

    #[test]
    fn escapes_error_html() {
        let html = render(&[st("companies", "failed")], "/admin/jobs");
        assert!(html.contains("&lt;boom&gt;"));
        assert!(!html.contains("<boom>"));
    }
}
