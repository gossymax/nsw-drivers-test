use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use dotenv::dotenv;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub headless: bool,
    /// Booking reference number used when managing an existing booking
    pub booking_id: String,
    /// Last name associated with the booking
    pub last_name: String,
    pub have_booking: bool,
    pub selenium_driver_url: String,
    pub selenium_element_timout: u64,
    pub selenium_element_polling: u64,
    pub retries: u64,
    /// How often to refresh scraping in minutes
    pub scrape_refresh_minutes: u64,
}

impl Settings {
    pub fn from_yaml<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();
        
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        let mut settings: Settings = serde_yaml::from_str(&contents)?;

        settings.booking_id = parse_env_var(&settings.booking_id)?;
        settings.last_name = parse_env_var(&settings.last_name)?;
        
        Ok(settings)
    }
}

fn parse_env_var(value: &str) -> Result<String, Box<dyn std::error::Error>> {
    if value.starts_with("${") && value.ends_with("}") {
        let env_name = &value[2..value.len() - 1];
        match env::var(env_name) {
            Ok(val) => Ok(val),
            Err(_) => Err(format!("Environment variable '{}' not found", env_name).into()),
        }
    } else {
        Ok(value.to_string())
    }
}
