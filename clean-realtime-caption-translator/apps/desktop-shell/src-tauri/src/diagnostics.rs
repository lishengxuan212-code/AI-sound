use crate::secret_redactor::redact_json;
use serde_json::Value;

pub fn diagnostic(prefix: &str, details: Value) {
    let safe = redact_json(details);
    println!("{} {}", prefix, safe);
}
