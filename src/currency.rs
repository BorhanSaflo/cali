use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use serde_json::Value;

// Currency exchange rate cache
#[derive(Debug, Clone)]
struct RateCache {
    rates: HashMap<String, HashMap<String, f64>>,
    timestamp: Instant,
}

impl RateCache {
    fn new() -> Self {
        Self {
            rates: HashMap::new(),
            timestamp: Instant::now(),
        }
    }
    
    fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }
}

// Global rate cache with mutex for thread safety
static RATE_CACHE: Lazy<Arc<Mutex<RateCache>>> = Lazy::new(|| {
    // Initialize with fallback rates
    let mut cache = RateCache::new();
    initialize_fallback_rates(&mut cache.rates);
    
    // Try to update with latest rates from API - no UI messages
    if let Ok(()) = fetch_latest_rates(&mut cache.rates) {
        // Reset timestamp if successful
        cache.timestamp = Instant::now();
    }
    
    Arc::new(Mutex::new(cache))
});

// Default TTL for cache entries (1 hour)
const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

// Fetch latest rates from a free API
fn fetch_latest_rates(rates: &mut HashMap<String, HashMap<String, f64>>) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    
    // Use the ExchangeRate-API free endpoint
    let response = client.get("https://open.er-api.com/v6/latest/USD")
        .timeout(Duration::from_secs(5))
        .send()?;
    
    let json: Value = response.json()?;
    
    // Check if the API call was successful
    if json["result"] != "success" {
        return Err("API call failed".into());
    }
    
    // Extract rates from the response
    if let Some(rates_obj) = json["rates"].as_object() {
        // First build USD rates
        let mut usd_rates = HashMap::new();
        usd_rates.insert("USD".to_string(), 1.0); // USD to USD is always 1.0
        
        for (currency, rate_value) in rates_obj {
            if let Some(rate) = rate_value.as_f64() {
                usd_rates.insert(currency.clone(), rate);
            }
        }
        
        // Store USD rates
        rates.insert("USD".to_string(), usd_rates.clone());
        
        // Now build rates for each other currency
        for (currency, usd_rate) in &usd_rates {
            if currency == "USD" {
                continue; // Already handled
            }
            
            let mut currency_rates = HashMap::new();
            currency_rates.insert(currency.clone(), 1.0); // Self rate is always 1.0
            
            for (target_currency, target_usd_rate) in &usd_rates {
                if target_currency == currency {
                    continue; // Skip self rate
                }
                
                // Convert through USD: currency → USD → target_currency
                let rate = target_usd_rate / usd_rate;
                currency_rates.insert(target_currency.clone(), rate);
            }
            
            rates.insert(currency.clone(), currency_rates);
        }
        
        return Ok(());
    }
    
    Err("Could not parse rates from API response".into())
}

// Fallback rates for when API is unavailable
fn initialize_fallback_rates(rates: &mut HashMap<String, HashMap<String, f64>>) {
    // USD rates
    let mut usd_rates = HashMap::new();
    usd_rates.insert("EUR".to_string(), 0.85);
    usd_rates.insert("GBP".to_string(), 0.72);
    usd_rates.insert("CAD".to_string(), 1.25);
    usd_rates.insert("JPY".to_string(), 115.0);
    usd_rates.insert("AUD".to_string(), 1.35);
    usd_rates.insert("CNY".to_string(), 6.45);
    usd_rates.insert("INR".to_string(), 75.0);
    usd_rates.insert("USD".to_string(), 1.0);
    rates.insert("USD".to_string(), usd_rates);
    
    // EUR rates
    let mut eur_rates = HashMap::new();
    eur_rates.insert("USD".to_string(), 1.18);
    eur_rates.insert("GBP".to_string(), 0.86);
    eur_rates.insert("CAD".to_string(), 1.47);
    eur_rates.insert("JPY".to_string(), 135.0);
    eur_rates.insert("AUD".to_string(), 1.59);
    eur_rates.insert("CNY".to_string(), 7.60);
    eur_rates.insert("INR".to_string(), 88.0);
    eur_rates.insert("EUR".to_string(), 1.0);
    rates.insert("EUR".to_string(), eur_rates);
    
    // GBP rates
    let mut gbp_rates = HashMap::new();
    gbp_rates.insert("USD".to_string(), 1.39);
    gbp_rates.insert("EUR".to_string(), 1.16);
    gbp_rates.insert("CAD".to_string(), 1.70);
    gbp_rates.insert("JPY".to_string(), 155.0);
    gbp_rates.insert("AUD".to_string(), 1.85);
    gbp_rates.insert("CNY".to_string(), 8.85);
    gbp_rates.insert("INR".to_string(), 102.0);
    gbp_rates.insert("GBP".to_string(), 1.0);
    rates.insert("GBP".to_string(), gbp_rates);
    
    // CAD rates
    let mut cad_rates = HashMap::new();
    cad_rates.insert("USD".to_string(), 0.80);
    cad_rates.insert("EUR".to_string(), 0.68);
    cad_rates.insert("GBP".to_string(), 0.59);
    cad_rates.insert("JPY".to_string(), 92.0);
    cad_rates.insert("AUD".to_string(), 1.10);
    cad_rates.insert("CNY".to_string(), 5.20);
    cad_rates.insert("INR".to_string(), 60.0);
    cad_rates.insert("CAD".to_string(), 1.0);
    rates.insert("CAD".to_string(), cad_rates);
}

// Function to calculate a rate for any currency pair
fn calculate_exchange_rate(from: &str, to: &str, rates: &HashMap<String, HashMap<String, f64>>) -> Option<f64> {
    // Direct conversion
    if let Some(from_rates) = rates.get(from) {
        if let Some(rate) = from_rates.get(to) {
            return Some(*rate);
        }
    }
    
    // Try to calculate via USD as base
    if from != "USD" && to != "USD" {
        if let (Some(from_usd), Some(usd_to)) = (
            rates.get("USD").and_then(|r| r.get(from)).map(|r| 1.0 / r),
            rates.get("USD").and_then(|r| r.get(to))
        ) {
            return Some(from_usd * usd_to);
        }
    }
    
    None
}

// Public function to get exchange rate, using cache when available
pub fn get_exchange_rate(from: &str, to: &str) -> Option<f64> {
    // If converting to the same currency, rate is always 1.0
    if from == to {
        return Some(1.0);
    }
    
    let mut cache = RATE_CACHE.lock().unwrap();
    
    // Check if we need to refresh the rates
    if cache.is_expired(CACHE_TTL) {
        // Try to update the rates from the API
        if let Ok(()) = fetch_latest_rates(&mut cache.rates) {
            cache.timestamp = Instant::now();
        }
    }
    
    calculate_exchange_rate(from, to, &cache.rates)
}

// Public function to manually update an exchange rate
// This allows users to set their own rates through expressions like:
// setrate USD to EUR = 0.92
pub fn set_exchange_rate(from: &str, to: &str, rate: f64) -> bool {
    if rate <= 0.0 {
        return false; // Invalid rate
    }
    
    let mut cache = RATE_CACHE.lock().unwrap();
    
    // Make sure we have entries for both currencies
    if !cache.rates.contains_key(from) {
        cache.rates.insert(from.to_string(), HashMap::new());
    }
    
    if !cache.rates.contains_key(to) {
        cache.rates.insert(to.to_string(), HashMap::new());
    }
    
    // Update the direct rate
    if let Some(from_rates) = cache.rates.get_mut(from) {
        from_rates.insert(to.to_string(), rate);
    }
    
    // Update the inverse rate
    if let Some(to_rates) = cache.rates.get_mut(to) {
        to_rates.insert(from.to_string(), 1.0 / rate);
    }
    
    true
} 