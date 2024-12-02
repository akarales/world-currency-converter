use crate::data::{GLOBAL_DATA, GlobalData};
use crate::models::{CurrencyInfo, CurrencyUpdateInfo};
use crate::errors::ServiceError;
use serde_json::json;
use std::fs;
use std::path::Path;
use log::debug;

pub async fn init_test_data() -> Result<(), ServiceError> {
    let test_data = json!({
        "last_checked": "2024-12-01T00:00:00Z",
        "last_modified": "2024-12-01T00:00:00Z",
        "data": {
            "Zimbabwe": {
                "primary_currency": "USD",
                "currencies": {
                    "USD": {
                        "name": "US Dollar",
                        "symbol": "$"
                    },
                    "ZWL": {
                        "name": "Zimbabwean Dollar",
                        "symbol": "Z$"
                    }
                },
                "is_multi_currency": true
            },
            "Panama": {
                "primary_currency": "USD",
                "currencies": {
                    "USD": {
                        "name": "US Dollar",
                        "symbol": "$"
                    },
                    "PAB": {
                        "name": "Panamanian Balboa",
                        "symbol": "B/."
                    }
                },
                "is_multi_currency": true
            },
            "France": {
                "primary_currency": "EUR",
                "currencies": {
                    "EUR": {
                        "name": "Euro",
                        "symbol": "€"
                    }
                },
                "is_multi_currency": true
            },
            "United States": {
                "primary_currency": "USD",
                "currencies": {
                    "USD": {
                        "name": "US Dollar",
                        "symbol": "$"
                    }
                },
                "is_multi_currency": true
            }
        }
    });

    // Ensure test directory exists
    let test_dir = Path::new("config/test");
    if !test_dir.exists() {
        fs::create_dir_all(test_dir)?;
    }

    // Write test data
    let test_file = test_dir.join("country_currencies.json");
    fs::write(&test_file, test_data.to_string())?;

    // Initialize global data if needed
    if GLOBAL_DATA.get().is_none() {
        let data = GlobalData::new("test_key".to_string());
        GLOBAL_DATA.set(data)
            .map_err(|_| ServiceError::InitializationError("Failed to set global data".to_string()))?;
    }

    Ok(())
}

pub async fn cleanup_test_env() -> Result<(), std::io::Error> {
    // Clean up test files
    let test_paths = [
        "config/test/country_currencies.json",
        "config/test/backups",
    ];

    for path in test_paths.iter() {
        let path = Path::new(path);
        if path.exists() {
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
    }

    debug!("Cleaned up test environment");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_init_and_cleanup() {
        // Test initialization
        init_test_data().await.expect("Failed to initialize test data");
        
        // Verify files were created
        let test_file = Path::new("config/test/country_currencies.json");
        assert!(test_file.exists());
        
        // Verify file contents
        let content = fs::read_to_string(test_file).unwrap();
        let data: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(data.get("data").unwrap().get("Zimbabwe").is_some());
        
        // Test cleanup
        cleanup_test_env().await.expect("Failed to cleanup test environment");
        assert!(!test_file.exists());
    }
}