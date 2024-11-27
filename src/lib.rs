pub mod registry;
pub mod handlers;
pub mod handlers_v1;
pub mod models;
pub mod cache;
pub mod config;
pub mod monitor;
pub mod rate_limit;
pub mod currency_service;
pub mod errors;
pub mod clients;

pub use errors::{ServiceError, ErrorResponse};

/// Formats a country name for consistent usage.
/// Capitalizes the first letter of each word and trims whitespace.
/// 
/// # Examples
/// ```
/// use currency_converter::format_country_name;
/// 
/// let formatted = format_country_name("united states");
/// assert_eq!(formatted, "United States");
/// 
/// let formatted = format_country_name("new zealand  ");
/// assert_eq!(formatted, "New Zealand");
/// ```
pub fn format_country_name(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut word = first.to_uppercase().collect::<String>();
                    word.extend(chars.map(|c| c.to_lowercase().next().unwrap_or(c)));
                    word
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// Rounds a number to two decimal places for currency display
/// 
/// # Examples
/// ```
/// use currency_converter::round_to_cents;
/// 
/// assert_eq!(round_to_cents(10.456), 10.46);
/// assert_eq!(round_to_cents(10.454), 10.45);
/// ```
pub fn round_to_cents(amount: f64) -> f64 {
    (amount * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CurrencyInfo, CountryInfo, CountryName};
    use std::collections::HashMap;

    #[test]
    fn test_format_country_name() {
        let test_cases = vec![
            ("united states", "United States"),
            ("JAPAN", "Japan"),
            ("new   zealand", "New Zealand"),
            ("great  britain", "Great Britain"),
            ("   france   ", "France"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(format_country_name(input), expected);
        }
    }

    #[test]
    fn test_round_to_cents() {
        let test_cases = vec![
            (10.456, 10.46),
            (10.454, 10.45),
            (0.0, 0.0),
            (99.999, 100.0),
            (-10.456, -10.46),
        ];

        for (input, expected) in test_cases {
            assert_eq!(round_to_cents(input), expected);
        }
    }

    #[test]
    fn test_country_info_creation() {
        let mut currencies = HashMap::new();
        currencies.insert(
            "USD".to_string(),
            CurrencyInfo {
                name: "United States Dollar".to_string(),
                symbol: "$".to_string(),
            },
        );

        let country = CountryInfo {
            name: CountryName {
                common: "United States".to_string(),
                official: "United States of America".to_string(),
            },
            currencies,
        };

        assert_eq!(country.name.common, "United States");
        assert_eq!(country.currencies.len(), 1);
        
        if let Some(usd) = country.currencies.get("USD") {
            assert_eq!(usd.symbol, "$");
            assert_eq!(usd.name, "United States Dollar");
        } else {
            panic!("USD currency not found");
        }
    }

    #[test]
    fn test_currency_formatting() {
        // Test that currency amounts are properly formatted
        assert_eq!(format!("{:.2}", round_to_cents(10.456)), "10.46");
        assert_eq!(format!("{:.2}", round_to_cents(10.454)), "10.45");
    }

    #[test]
    fn test_currency_math() {
        // Test basic currency calculations
        let amount = 100.0;
        let rate = 0.85;
        let converted = round_to_cents(amount * rate);
        assert_eq!(converted, 85.0);

        // Test that we handle floating point precision correctly
        let amount = 33.33;
        let rate = 1.2;
        let converted = round_to_cents(amount * rate);
        assert_eq!(converted, 40.0);
    }
}