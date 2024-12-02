use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;
use lazy_static::lazy_static;
use std::sync::Once;
use colored::Colorize;

lazy_static! {
    static ref TEST_RESULTS: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
    static ref TOTAL_TESTS: AtomicUsize = AtomicUsize::new(0);
    static ref PASSED_TESTS: AtomicUsize = AtomicUsize::new(0);
    static ref INIT: Once = Once::new();
}

pub async fn record_test_result(name: &str, passed: bool) {
    let mut results = TEST_RESULTS.lock().await;
    results.insert(name.to_string(), passed);
    TOTAL_TESTS.fetch_add(1, Ordering::SeqCst);
    if passed {
        PASSED_TESTS.fetch_add(1, Ordering::SeqCst);
    }
}

pub async fn print_test_summary() {
    let results = TEST_RESULTS.lock().await;
    let total = TOTAL_TESTS.load(Ordering::SeqCst);
    let passed = PASSED_TESTS.load(Ordering::SeqCst);
    let failed = total - passed;

    println!("\n{}", "=== Currency Manager Test Summary ===".bold());
    println!("{}: {}", "Total Tests Run".bold(), total);
    println!("{}: {}", "Tests Passed".bold().green(), passed);
    println!("{}: {}", "Tests Failed".bold().red(), failed);
    println!("\n{}", "Detailed Results:".bold());

    let mut categories: HashMap<&str, Vec<(&str, bool)>> = HashMap::new();
    
    for (test_name, passed) in results.iter() {
        let category = if test_name.contains("currency_manager") {
            "Currency Manager"
        } else if test_name.contains("multi_currency") {
            "Multi-Currency Support"
        } else if test_name.contains("backup") {
            "Backup Operations"
        } else if test_name.contains("config") {
            "Configuration"
        } else if test_name.contains("exchange") {
            "Exchange Rates"
        } else {
            "Other Tests"
        };

        categories.entry(category).or_default().push((test_name, *passed));
    }

    for (category, tests) in categories.iter() {
        println!("\n{}:", category.bold().blue());
        for (test_name, passed) in tests {
            let status = if *passed {
                "✓".green().bold()
            } else {
                "✗".red().bold()
            };
            println!("  {} {}", status, test_name);
        }
    }

    println!("\n{}", "=== End Test Summary ===".bold());
}