use crate::models::normalize_key;

const BLOCKED_DOMAINS: [&str; 6] = [".ru", ".com", ".net", ".gg", "t.me", "discord.gg"];

/// Полностью очищает ввод от управляющих символов и сырых символов секции `§`
/// (защита от подделки системных сообщений и инъекций форматирования).
pub fn sanitize_input(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '§' {
            // Удаляем сам символ '§' и следующий за ним цветовой/стилевой код
            if let Some(&next) = chars.peek() {
                if next.is_ascii_alphanumeric() {
                    let _ = chars.next();
                }
            }
            continue;
        }
        if c.is_control() {
            continue;
        }
        out.push(c);
    }
    out.trim().to_string()
}

/// Преобразует `&`-коды в `§`-коды только при наличии права `kronos-ranks:chat.color`.
pub fn apply_color_codes(sanitized: &str, allow_colors: bool) -> String {
    if !allow_colors {
        return sanitized.to_string();
    }
    let mut out = String::with_capacity(sanitized.len());
    let mut chars = sanitized.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if let Some(&next) = chars.peek() {
                let low = next.to_ascii_lowercase();
                if matches!(low, '0'..='9' | 'a'..='f' | 'l' | 'm' | 'n' | 'o' | 'r') {
                    out.push('§');
                    out.push(chars.next().unwrap());
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

/// Анти-капс: если длина сообщения > 5 символов и > 60% букв заглавные — приводит к нижнему регистру.
pub fn apply_anti_caps(text: &str) -> String {
    if text.chars().count() <= 5 {
        return text.to_string();
    }
    let mut letters = 0usize;
    let mut upper = 0usize;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            letters += 1;
            if ch.is_uppercase() {
                upper += 1;
            }
        }
    }
    if letters > 0 && (upper * 100) > (letters * 60) {
        text.to_lowercase()
    } else {
        text.to_string()
    }
}

/// Анти-флуд: возвращает `true`, если новое сообщение совпадает с предыдущим.
pub fn is_duplicate_flood(last_msg: Option<&str>, new_msg: &str) -> bool {
    match last_msg {
        Some(prev) => normalize_key(prev) == normalize_key(new_msg),
        None => false,
    }
}

/// Проверяет остаток кулдауна. Возвращает `Some(seconds_left)` если кулдаун ещё активен.
pub fn remaining_cooldown_secs(
    last_sent_ms: Option<u64>,
    now_ms: u64,
    cooldown_ms: u64,
) -> Option<u64> {
    if cooldown_ms == 0 {
        return None;
    }
    let last = last_sent_ms?;
    let elapsed = now_ms.saturating_sub(last);
    if elapsed < cooldown_ms {
        Some((cooldown_ms - elapsed).div_ceil(1000))
    } else {
        None
    }
}

/// Анти-реклама: проверяет наличие IPv4-адресов и запрещённых доменных зон/ссылок.
pub fn contains_advertisement(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if contains_ipv4(&lower) {
        return true;
    }
    for token in lower.split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']' | ',' | ';' | '"' | '\'')) {
        let clean = token.trim_matches(|c: char| matches!(c, '.' | '/' | ':' | '!' | '?'));
        if clean.is_empty() {
            continue;
        }
        if clean.contains("http://") || clean.contains("https://") {
            return true;
        }
        for pattern in BLOCKED_DOMAINS {
            if pattern.starts_with('.') {
                if let Some(pos) = clean.find(pattern) {
                    let after = &clean[pos + pattern.len()..];
                    let has_host_before = pos > 0
                        && clean[..pos].chars().any(|c| c.is_ascii_alphanumeric());
                    let boundary_ok = after.is_empty()
                        || after.starts_with('/')
                        || after.starts_with(':')
                        || after.starts_with('?');
                    if has_host_before && boundary_ok {
                        return true;
                    }
                }
            } else if clean.contains(pattern) {
                return true;
            }
        }
    }
    false
}

/// Строгий детектор IPv4 (`a.b.c.d`, где каждый октет 0..=255), без ложных срабатываний на числа.
fn contains_ipv4(text: &str) -> bool {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let candidate = &text[start..i];
            let parts: Vec<&str> = candidate.split('.').collect();
            if parts.len() == 4
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.len() <= 3 && p.parse::<u8>().is_ok())
            {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}
