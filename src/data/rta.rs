use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thirtyfour::components::SelectElement;
use thirtyfour::{By, DesiredCapabilities, WebDriver};
use thirtyfour::prelude::*;
use rand::Rng;

use crate::settings::Settings;
use super::shared_booking::{LocationBookings, TimeSlot};

async fn random_sleep(min_millis: u64, max_millis: u64) {
    if min_millis >= max_millis {
        tokio::time::sleep(Duration::from_millis(min_millis)).await;
        return;
    }
    let duration = rand::thread_rng().gen_range(min_millis..max_millis);
    tokio::time::sleep(Duration::from_millis(duration)).await;
}

async fn type_like_human(element: &WebElement, text: &str, min_delay_ms: u64, max_delay_ms: u64) -> WebDriverResult<()> {
    for char in text.chars() {
        element.send_keys(char.to_string()).await?;
        random_sleep(min_delay_ms, max_delay_ms).await;
    }
    Ok(())
}

pub async fn scrape_rta_timeslots(
    locations: Vec<String>,
    settings: &Settings
) -> WebDriverResult<HashMap<String, LocationBookings>> {

    let mut location_bookings: HashMap<String, LocationBookings> = HashMap::new();

    let mut caps = DesiredCapabilities::chrome();
    if settings.headless {
        caps.add_arg("--headless=new")?;
    }
    caps.add_arg("--no-sandbox")?;
    caps.add_arg("--disable-dev-shm-usage")?;
    caps.add_arg("--window-size=1920,1080")?;
    caps.add_arg("--start-maximized")?;
    caps.add_arg("--user-agent=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.114 Safari/537.36")?;
    caps.add_arg("--disable-blink-features=AutomationControlled")?;
    caps.add_experimental_option("excludeSwitches", vec!["enable-automation"]);
    caps.add_experimental_option("useAutomationExtension", false);


    let driver = WebDriver::new(settings.selenium_driver_url.clone(), caps).await?;

    driver.execute(r#"
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
        // Minimal spoofing of window.chrome, might need adjustment
        window.chrome = window.chrome || {};
        window.chrome.runtime = window.chrome.runtime || {};
        // Attempt to remove cdc_ properties (might not exist)
        try {
            let key = Object.keys(window).find(key => key.startsWith('cdc_'));
            if (key) { delete window[key]; }
            let docKey = Object.keys(document).find(key => key.startsWith('cdc_'));
            if (docKey) { delete document[docKey]; }
        } catch (e) { console.debug('Error removing cdc keys:', e); }
    "#, Vec::new()).await?;


    let timeout = Duration::from_millis(settings.selenium_element_timout);
    let polling = Duration::from_millis(settings.selenium_element_polling);

    driver.goto("https://www.myrta.com/wps/portal/extvp/myrta/login/").await?;
    random_sleep(1000, 2000).await;

    // Use booking id and last name for login when modifying an existing booking
    let booking_input = driver.query(By::Id("widget_bookingId")).first().await?;
    booking_input.wait_until().wait(timeout, polling).displayed().await?;
    random_sleep(200, 500).await;
    type_like_human(&booking_input, &settings.booking_id, 60, 180).await?;
    random_sleep(300, 700).await;

    let last_name_input = driver.query(By::Id("widget_lastName")).first().await?;
    last_name_input.wait_until().wait(timeout, polling).displayed().await?;
    random_sleep(200, 500).await;
    type_like_human(&last_name_input, &settings.last_name, 60, 180).await?;
    random_sleep(400, 800).await;

    let next_button = driver.query(By::Id("nextButton")).first().await?;
    next_button.wait_until().wait(timeout, polling).displayed().await?;
    // next_button.wait_until().wait(timeout, polling).has_attribute("aria-disabled", "false").await?; // Alternative if clickable() doesn't work
    random_sleep(250, 600).await;
    next_button.click().await?;

    random_sleep(2000, 4000).await;

    if settings.have_booking {
        let manage_booking = driver.query(By::XPath("//*[text()=\"Manage booking\"]")).first().await?;
        manage_booking.wait_until().wait(timeout, polling).displayed().await?;
        random_sleep(200, 500).await;
        manage_booking.click().await?;
        random_sleep(1500, 2500).await;

        let change_location = driver.query(By::Id("changeLocationButton")).first().await?;
        change_location.wait_until().wait(timeout, polling).displayed().await?;
        random_sleep(200, 500).await;
        change_location.click().await?;
        random_sleep(1000, 2000).await;

    } else {
         let book_test = driver.query(By::XPath("//*[text()=\"Book test\"]")).first().await?;
         book_test.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         book_test.click().await?;
         random_sleep(1500, 2500).await;

         let car_option = driver.query(By::Id("CAR")).first().await?;
         car_option.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         car_option.click().await?;
         random_sleep(500, 1000).await;

         let test_item = driver.query(By::XPath("//fieldset[@id='DC']/span[contains(@class, 'rms_testItemResult')]")).first().await?;
         test_item.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         test_item.click().await?;
         random_sleep(500, 1000).await;

         let next_button = driver.query(By::Id("nextButton")).first().await?;
         next_button.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         next_button.click().await?;
         random_sleep(1500, 2500).await;

         let check_terms = driver.query(By::Id("checkTerms")).first().await?;
         check_terms.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(100, 300).await;
         check_terms.click().await?;
         random_sleep(500, 1000).await;

         let next_button_terms = driver.query(By::Id("nextButton")).first().await?;
         next_button_terms.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         next_button_terms.click().await?;
         random_sleep(1000, 2000).await;
    }

    for location in locations {
        println!("INFO: Processing location: {}", location);
        let process_result: WebDriverResult<LocationBookings> = async {

            random_sleep(1000, 2000).await;

            let location_select_dropdown = driver.query(By::Id("rms_batLocLocSel")).first().await?;
            location_select_dropdown.wait_until().wait(timeout, polling).displayed().await?;
            random_sleep(200, 400).await;
            location_select_dropdown.click().await?;
            random_sleep(500, 1000).await;

            let select_element_query = driver.query(By::Id("rms_batLocationSelect2"));
            let select_element = select_element_query.wait(timeout, polling).first().await?;
            select_element.wait_until().wait(timeout, polling).displayed().await?;
            let select_box = SelectElement::new(&select_element).await?;

            if let Err(e) = select_box.select_by_value(&location).await {
                 eprintln!("ERROR: Failed to select location '{}' in dropdown: {}. Ensure the value is correct.", location, e);
                 return Err(e);
            }

            println!("INFO: Selected location: {}", location);
            random_sleep(2500, 4000).await;

            let next_button_loc = driver.query(By::Id("nextButton")).first().await?;
            next_button_loc.wait_until().wait(timeout, polling).displayed().await?;
            random_sleep(200, 500).await;
            next_button_loc.click().await?;

            random_sleep(1000, 2000).await;

            match driver.query(By::Id("getEarliestTime")).first().await {
                Ok(element) => {
                     if element.is_clickable().await.unwrap_or(false) {
                         println!("INFO: Found 'Get Earliest Time' button, attempting click.");
                         random_sleep(200, 400).await;
                         if let Err(e) = element.click().await {
                            eprintln!("WARN: Failed to click 'Get Earliest Time' button for {}: {}. Proceeding anyway.", location, e);
                         } else {
                             println!("INFO: Clicked 'Get Earliest Time'.");
                             random_sleep(2500, 4500).await;
                         }
                     } else {
                         println!("INFO: 'Get Earliest Time' button found but not clickable (visible/enabled).");
                         random_sleep(500, 1000).await;
                     }
                },
                Err(_) => {
                    println!("INFO: 'Get Earliest Time' button not found for {}. Proceeding.", location);
                    random_sleep(500, 1000).await;
                },
            }

            random_sleep(1000, 2500).await;

            let timeslots = driver.execute("return timeslots", vec![]).await?;

            let next_available_date = timeslots.json()
                .get("ajaxresult")
                .and_then(|ajax| ajax.get("slots"))
                .and_then(|slots| slots.get("nextAvailableDate"))
                .and_then(|date| date.as_str())
                .map(|s| s.to_string());
                
            let slots: Vec<TimeSlot> = timeslots.json()
                .get("ajaxresult")
                .and_then(|ajax| ajax.get("slots"))
                .and_then(|slots| slots.get("listTimeSlot"))
                .and_then(|list| serde_json::from_value(list.clone()).ok())
                .unwrap_or_else(Vec::new);


            println!("INFO: Parsed {} slots for {}. Next available: {:?}", slots.len(), location, next_available_date);

            let location_result = LocationBookings {
                location: location.to_string(),
                slots,
                next_available_date,
            };

            random_sleep(800, 1500).await;

            let another_location_link = driver.query(By::Id("anotherLocationLink")).first().await?;
            another_location_link.wait_until().wait(timeout, polling).displayed().await?;
            random_sleep(200, 500).await;
            another_location_link.click().await?;

            Ok(location_result)

        }.await;

        match process_result {
            Ok(booking_data) => {
                location_bookings.insert(location.clone(), booking_data);
            }
            Err(e) => {
                 eprintln!("ERROR: Failed processing location {}: {}", location, e);
                 match driver.query(By::Id("anotherLocationLink")).first().await {
                     Ok(link) => {
                          if link.is_displayed().await.unwrap_or(false) {
                              eprintln!("INFO: Attempting recovery click on 'Another Location'.");
                              if let Err(click_err) = link.click().await {
                                  eprintln!("WARN: Recovery click failed: {}", click_err);
                              } else {
                                  println!("INFO: Recovery click succeeded.");
                              }
                          } else {
                              eprintln!("WARN: Recovery link found but not displayed.");
                          }
                     }
                     Err(_) => {
                         eprintln!("WARN: Recovery link ('anotherLocationLink') not found. State unclear.");
                     }
                 }
                 random_sleep(2000, 3000).await;
                 continue;
            }
        }
         random_sleep(1500, 3000).await;
    }

    println!("INFO: Finished scraping all locations. Quitting driver.");
    driver.quit().await?;

    Ok(location_bookings)
}

/// Search approved locations for a slot before a given date and attempt to book it.
/// The booking process is highly dependent on the Service NSW website and may
/// require adjusting the element selectors.
pub async fn book_first_available(
    locations: Vec<String>,
    before: chrono::NaiveDate,
    settings: &Settings,
) -> WebDriverResult<Option<(String, String)>> {
    let bookings = scrape_rta_timeslots(locations.clone(), settings).await?;

    for (loc, info) in bookings {
        if let Some(slot) = info
            .slots
            .iter()
            .filter(|s| s.availability)
            .find(|s| {
                chrono::NaiveDateTime::parse_from_str(&s.start_time, "%d/%m/%Y %H:%M")
                    .map(|dt| dt.date() <= before)
                    .unwrap_or(false)
            })
        {
            // TODO: implement DOM interaction to select the slot and confirm the booking
            println!("Would attempt to book {} at {}", loc, slot.start_time);
            return Ok(Some((loc, slot.start_time.clone())));
        }
    }

    println!("No available slots before {} found in approved locations", before);
    Ok(None)
}
