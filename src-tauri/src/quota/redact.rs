pub fn mask_secret(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        return "****".to_string();
    }

    let start: String = chars.iter().take(3).collect();
    let end: String = chars
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}****{}", start, end)
}

pub fn redact_sensitive(message: &str) -> String {
    let mut output = message.to_string();
    for marker in ["Authorization", "management_key", "managementKey", "Bearer"] {
        if output.contains(marker) {
            output = output.replace(marker, "[redacted]");
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::mask_secret;

    #[test]
    fn masks_secret() {
        assert_eq!(mask_secret("sk-1234567890"), "sk-****7890");
        assert_eq!(mask_secret("short"), "****");
    }
}
