use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryCurrencyConfig {
    pub primary_currency: String,
    pub currencies: Vec<String>,
    pub is_multi_currency: bool,
}

lazy_static! {
    pub static ref COUNTRY_CURRENCY_DATA: HashMap<String, CountryCurrencyConfig> = {
        let data = include_str!("../config/country_currencies.json");
        serde_json::from_str(data).expect("Failed to parse country currency data")
    };
}