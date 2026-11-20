use std::fmt;

/// 1.00 монета = 100 минимальных единиц (fixed-point u64 без погрешностей f64).
pub const MINOR_SCALE: u64 = 100;
/// Стартовый баланс нового игрока: 100.00 монет.
pub const DEFAULT_STARTING_BALANCE: u64 = 100 * MINOR_SCALE;

pub fn normalize_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

pub fn is_valid_player_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty() && trimmed.len() <= 32 && !trimmed.chars().any(char::is_whitespace)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomyError {
    NegativeAmount,
    ZeroAmount,
    InvalidAmount,
    SelfTransfer,
    InvalidPlayerName,
    InsufficientFunds { available: u64, required: u64 },
    Overflow,
}

impl fmt::Display for EconomyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EconomyError::NegativeAmount => write!(f, "Сумма не может быть отрицательной."),
            EconomyError::ZeroAmount => write!(f, "Сумма должна быть больше 0."),
            EconomyError::InvalidAmount => write!(
                f,
                "Некорректный формат суммы. Пример: 100 или 25.50 (до 2 знаков после точки)."
            ),
            EconomyError::SelfTransfer => write!(f, "Нельзя переводить монеты самому себе."),
            EconomyError::InvalidPlayerName => write!(f, "Указано некорректное имя игрока."),
            EconomyError::InsufficientFunds { available, required } => write!(
                f,
                "Недостаточно средств! Требуется: {}, доступно: {}.",
                format_amount(*required),
                format_amount(*available)
            ),
            EconomyError::Overflow => {
                write!(f, "Операция отклонена: превышен максимальный лимит баланса.")
            }
        }
    }
}

/// Парсер суммы из строки в сотые доли (минимальные единицы `u64`).
pub fn parse_amount(raw: &str, allow_zero: bool) -> Result<u64, EconomyError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(EconomyError::InvalidAmount);
    }
    if s.starts_with('-') {
        return Err(EconomyError::NegativeAmount);
    }
    if s.starts_with('+') {
        return Err(EconomyError::InvalidAmount);
    }

    let normalized = s.replace(',', ".");
    let mut parts = normalized.split('.');
    let whole_str = parts.next().unwrap_or("");
    let frac_opt = parts.next();
    if parts.next().is_some() {
        return Err(EconomyError::InvalidAmount);
    }

    if whole_str.is_empty() || !whole_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(EconomyError::InvalidAmount);
    }

    let whole: u64 = whole_str.parse().map_err(|_| EconomyError::Overflow)?;
    let whole_minor = whole
        .checked_mul(MINOR_SCALE)
        .ok_or(EconomyError::Overflow)?;

    let frac_minor: u64 = match frac_opt {
        None => 0,
        Some(frac_str) => {
            if frac_str.is_empty() || frac_str.len() > 2 || !frac_str.bytes().all(|b| b.is_ascii_digit()) {
                return Err(EconomyError::InvalidAmount);
            }
            let parsed: u64 = frac_str.parse().map_err(|_| EconomyError::InvalidAmount)?;
            if frac_str.len() == 1 {
                parsed * 10
            } else {
                parsed
            }
        }
    };

    let total = whole_minor
        .checked_add(frac_minor)
        .ok_or(EconomyError::Overflow)?;

    if !allow_zero && total == 0 {
        return Err(EconomyError::ZeroAmount);
    }

    Ok(total)
}

pub fn format_thousands_u128(value: u128) -> String {
    let digits = value.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3);
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (len - idx) % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

pub fn format_amount(minor_units: u64) -> String {
    let whole = (minor_units / MINOR_SCALE) as u128;
    let frac = minor_units % MINOR_SCALE;
    let whole_fmt = format_thousands_u128(whole);
    if frac == 0 {
        whole_fmt
    } else {
        format!("{whole_fmt}.{frac:02}")
    }
}

pub fn format_amount_u128(minor_units: u128) -> String {
    let scale = MINOR_SCALE as u128;
    let whole = minor_units / scale;
    let frac = minor_units % scale;
    let whole_fmt = format_thousands_u128(whole);
    if frac == 0 {
        whole_fmt
    } else {
        format!("{whole_fmt}.{frac:02}")
    }
}
