use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;

// --- Logging JSON ---

#[derive(Debug, Serialize)]
pub(crate) struct LogEntry {
    pub(crate) event: String,
    pub(crate) timestamp: String,
    pub(crate) pid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ssh_client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) args: Option<HashMap<String, String>>,
    pub(crate) effective_tags: Vec<String>,
    pub(crate) action_tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) defaults_applied: Vec<String>,
}

impl LogEntry {
    pub(crate) fn new(event: &str) -> Self {
        LogEntry {
            event: event.to_string(),
            timestamp: now_iso8601(),
            pid: std::process::id(),
            domain: None,
            action: None,
            reason: None,
            ssh_client: None,
            duration_ms: None,
            session_id: None,
            args: None,
            effective_tags: Vec::new(),
            action_tags: Vec::new(),
            defaults_applied: Vec::new(),
        }
    }

    pub(crate) fn with_domain(mut self, domain: &str) -> Self {
        self.domain = Some(domain.to_string());
        self
    }

    pub(crate) fn with_action(mut self, action: &str) -> Self {
        self.action = Some(action.to_string());
        self
    }

    pub(crate) fn with_reason(mut self, reason: &str) -> Self {
        self.reason = Some(reason.to_string());
        self
    }

    pub(crate) fn with_ssh_client(mut self, client: &str) -> Self {
        self.ssh_client = Some(client.to_string());
        self
    }

    pub(crate) fn to_json(&self) -> String {
        // INVARIANT: LogEntry est toujours serializable
        serde_json::to_string(self).expect("LogEntry serialization cannot fail")
    }
}

fn now_iso8601() -> String {
    // Format simplifie sans dependance chrono : YYYY-MM-DDTHH:MM:SSZ via /proc/driver/rtc
    // Fallback sur un timestamp secondes si indisponible
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Conversion naive epoch -> ISO 8601
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calcul de la date depuis epoch (1970-01-01)
    let (year, month, day) = epoch_days_to_ymd(days);

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn epoch_days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithme simplifie pour convertir jours depuis epoch en Y-M-D
    let mut y = 1970;
    let mut remaining = days;

    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }

    let month_days: [u64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md {
            m = i as u64 + 1;
            break;
        }
        remaining -= md;
    }
    if m == 0 {
        m = 12;
    }

    (y, m, remaining + 1)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

/// Ecrit une entree de log dans le fichier specifie
#[must_use = "log write result must be checked"]
pub(crate) fn write_log(path: &str, entry: &LogEntry) -> Result<(), String> {
    let parent = std::path::Path::new(path)
        .parent()
        .ok_or_else(|| "invalid log path".to_string())?;

    if !parent.exists() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create log dir: {e}"))?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("cannot open log file: {e}"))?;

    writeln!(file, "{}", entry.to_json()).map_err(|e| format!("cannot write log: {e}"))
}
