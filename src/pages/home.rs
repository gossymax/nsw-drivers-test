use std::collections::HashMap;
use std::time::Duration;

use leptos::prelude::*;
use leptos::server_fn::error::NoCustomError;
use reqwest::header;
use serde::{Deserialize, Serialize};
use web_sys::wasm_bindgen::prelude::Closure;

use crate::data::location::LocationManager;
use crate::data::shared_booking::TimeSlot;
use crate::utils::date::TimeDisplay;
use crate::utils::geocoding::geocode_address;
use crate::pages::location_table::LocationsTable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationBookingViewModel {
    pub location: String,
    pub earliest_slot: Option<TimeSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingResponse {
    pub bookings: Vec<LocationBookingViewModel>,
    pub last_updated: Option<String>,
    pub etag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationDetailBookingResponse {
    pub location: String,
    pub slots: Vec<TimeSlot>,
    pub etag: String,
}

#[server(GetBookings)]
pub async fn get_location_bookings(
    client_etag: String,
) -> Result<Option<BookingResponse>, ServerFnError> {
    use crate::data::booking::BookingManager;
    use axum::http::HeaderValue;
    use axum::http::StatusCode;

    let response = expect_context::<leptos_axum::ResponseOptions>();

    let (booking_data, server_etag) = BookingManager::get_data();
    if client_etag == server_etag {
        // WARN: for some reason this makes it open in hte browser
        // response.set_status(StatusCode::NOT_MODIFIED);
        return Ok(None);
    }

    let view_models: Vec<_> = booking_data
        .results
        .iter()
        .map(|location_booking| {
            let earliest_slot = location_booking
                .slots
                .iter()
                .filter(|slot| slot.availability)
                .min_by(|a, b| a.start_time.cmp(&b.start_time))
                .cloned();

            LocationBookingViewModel {
                location: location_booking.location.clone(),
                earliest_slot,
            }
        })
        .collect();

    Ok(Some(BookingResponse {
        bookings: view_models,
        last_updated: booking_data.last_updated.clone(),
        etag: server_etag,
    }))
}

#[server(GetLocationDetails)]
pub async fn get_location_details(
    location_id: String,
    client_etag: String,
) -> Result<Option<LocationDetailBookingResponse>, ServerFnError> {
    use crate::data::booking::BookingManager;

    let (location_booking, server_etag) = BookingManager::get_location_data(location_id).ok_or(
        ServerFnError::<NoCustomError>::ServerError("Location not found".into()),
    )?;

    if client_etag == server_etag {
        // WARN: for some reason this makes it open in hte browser
        // response.set_status(StatusCode::NOT_MODIFIED);
        return Ok(None);
    }

    Ok(Some(LocationDetailBookingResponse {
        location: location_booking.location,
        slots: location_booking.slots,
        etag: server_etag,
    }))
}

#[server(FindFirstSlot)]
pub async fn find_first_slot(
    before: String,
    booking_id: String,
    last_name: String,
) -> Result<Option<(String, String)>, ServerFnError> {
    use crate::data::booking::BookingManager;
    use crate::data::rta::book_first_available;
    use crate::settings::Settings;

    let date = chrono::NaiveDate::parse_from_str(&before, "%Y-%m-%d")
        .map_err(|e| ServerFnError::<NoCustomError>::ServerError(e.to_string()))?;

    let mut settings = Settings::from_yaml("settings.yaml")
        .map_err(|e| ServerFnError::<NoCustomError>::ServerError(e.to_string()))?;
    settings.booking_id = booking_id;
    settings.last_name = last_name;

    let locations: Vec<String> = BookingManager::get_data()
        .0
        .results
        .iter()
        .map(|l| l.location.clone())
        .collect();

    match book_first_available(locations, date, &settings).await {
        Ok(res) => Ok(res),
        Err(e) => Err(ServerFnError::<NoCustomError>::ServerError(e.to_string())),
    }
}


#[server(StartAutoFind)]
pub async fn start_auto_find(
    before: String,
    booking_id: String,
    last_name: String,
    locations: Vec<String>,
) -> Result<(), ServerFnError> {
    use crate::data::booking::BookingManager;
    use crate::settings::Settings;

    let date = chrono::NaiveDate::parse_from_str(&before, "%Y-%m-%d")
        .map_err(|e| ServerFnError::<NoCustomError>::ServerError(e.to_string()))?;

    let mut settings = Settings::from_yaml("settings.yaml")
        .map_err(|e| ServerFnError::<NoCustomError>::ServerError(e.to_string()))?;
    settings.booking_id = booking_id;
    settings.last_name = last_name;

    BookingManager::start_auto_find(locations, date, settings);
    Ok(())
}

#[server(StopAutoFind)]
pub async fn stop_auto_find() -> Result<(), ServerFnError> {
    use crate::data::booking::BookingManager;
    BookingManager::stop_auto_find();
    Ok(())
}

#[server(GetAutoFindStatus)]
pub async fn get_auto_find_status() -> Result<bool, ServerFnError> {
    use crate::data::booking::BookingManager;
    Ok(BookingManager::auto_find_running())
}


#[component]
pub fn HomePage() -> impl IntoView {
    let (address_input, set_address_input) = create_signal(String::new());
    let (latitude, set_latitude) = create_signal(-33.8688197);
    let (longitude, set_longitude) = create_signal(151.2092955);
    let (current_location_name, set_current_location_name) = create_signal("Sydney".to_string());
    let (geocoding_status, set_geocoding_status) = create_signal::<Option<String>>(None);
    let (is_loading, set_is_loading) = create_signal(false);

    let (last_updated, set_last_updated) = create_signal::<Option<String>>(None);

    let (bookings, set_bookings) = create_signal(Vec::<LocationBookingViewModel>::new());
    let (is_fetching_bookings, set_is_fetching_bookings) = create_signal(false);

    let (booking_etag, set_booking_etag) = create_signal(String::new());

    // inputs for booking search
    let (booking_id_input, set_booking_id_input) = create_signal(String::new());
    let (last_name_input, set_last_name_input) = create_signal(String::new());
    let (latest_date_input, set_latest_date_input) = create_signal(String::new());
    let (find_slot_msg, set_find_slot_msg) = create_signal::<Option<String>>(None);


    // auto finder state
    let (show_auto_panel, set_show_auto_panel) = create_signal(false);
    let (auto_active, set_auto_active) = create_signal(false);
    let (selected_locations, set_selected_locations) = create_signal(Vec::<String>::new());
    let (auto_msg, set_auto_msg) = create_signal::<Option<String>>(None);


    let (reset_sort_trigger, set_reset_sort_trigger) = create_signal(());

    let location_manager = LocationManager::new();

    let fetch_bookings = move || {
        set_is_fetching_bookings(true);

        leptos::task::spawn_local(async move {
            match get_location_bookings(booking_etag.get_untracked()).await {
                Ok(data) => {
                    match data {
                        Some(data) => {
                            set_bookings(data.bookings);
                            set_last_updated(data.last_updated);
                            set_booking_etag(data.etag);
                        }
                        None => {}
                    };
                }
                Err(err) => {
                    leptos::logging::log!("Error fetching bookings: {:?}", err);
                }
            }
            set_is_fetching_bookings(false);
        });
    };

#[cfg(not(feature = "ssr"))]
fetch_bookings();

#[cfg(not(feature = "ssr"))]
leptos::task::spawn_local(async move {
    if let Ok(active) = get_auto_find_status().await {
        set_auto_active(active);
    }
});

    #[cfg(not(feature = "ssr"))]
    Effect::new(move |_| {
        leptos::logging::log!("Setting up client-side refresh mechanism");

        let handle = set_interval_with_handle(
            move || {
                leptos::logging::log!("Triggering refresh");
                fetch_bookings();
            },
            Duration::from_secs(1200),
        )
        .expect("failed to set interval");

        on_cleanup(move || {
            handle.clear();
        });

        || {}
    });

    let handle_geocode = move |_| {
        let address = address_input.get();
        if address.is_empty() {
            set_geocoding_status(Some("Please enter a location".to_string()));
            return;
        }

        set_geocoding_status(Some("Searching...".to_string()));
        set_is_loading(true);

        leptos::task::spawn_local(async move {
            match geocode_address(&address).await {
                Ok(result) => {
                    set_latitude(result.latitude);
                    set_longitude(result.longitude);
                    set_current_location_name(result.display_name);
                    set_geocoding_status(None);
                    set_is_loading(false);
                    set_reset_sort_trigger(());
                }
                Err(err) => {
                    set_geocoding_status(Some(format!("Error: {}", err)));
                    set_is_loading(false);
                }
            }
        });
    };

    let handle_find_slot = move |_| {
        let booking = booking_id_input.get();
        let last = last_name_input.get();
        let date = latest_date_input.get();

        if booking.is_empty() || last.is_empty() || date.is_empty() {
            set_find_slot_msg(Some("Please fill in all fields".to_string()));
            return;
        }

        set_find_slot_msg(Some("Searching...".to_string()));
        leptos::task::spawn_local(async move {
            match find_first_slot(date.clone(), booking, last).await {
                Ok(Some((loc, time))) => {
                    set_find_slot_msg(Some(format!("Found slot at {} on {}", loc, time)));
                }
                Ok(None) => {
                    set_find_slot_msg(Some("No slot found".to_string()));
                }
                Err(e) => {
                    set_find_slot_msg(Some(format!("Error: {e}")));
                }
            }
        });
    };


    let toggle_location = move |loc: String| {
        let mut current = selected_locations.get();
        if let Some(pos) = current.iter().position(|l| l == &loc) {
            current.remove(pos);
        } else {
            current.push(loc);
        }
        set_selected_locations(current);
    };

    let toggle_auto_panel = move |_| {
        set_show_auto_panel(!show_auto_panel.get());
    };

    let handle_auto_action = move |_| {
        let booking = booking_id_input.get();
        let last = last_name_input.get();
        let date = latest_date_input.get();
        let locs = selected_locations.get();

        if booking.is_empty() || last.is_empty() || date.is_empty() || locs.is_empty() {
            set_auto_msg(Some("Please fill in details and pick locations".into()));
            return;
        }

        set_auto_msg(Some("Processing...".into()));
        if auto_active.get() {
            leptos::task::spawn_local(async move {
                if let Err(e) = stop_auto_find().await {
                    set_auto_msg(Some(format!("Error: {e}")));
                } else {
                    set_auto_msg(Some("Auto finder stopped".into()));
                    set_auto_active(false);
                }
            });
        } else {
            leptos::task::spawn_local(async move {
                if let Err(e) = start_auto_find(date.clone(), booking, last, locs).await {
                    set_auto_msg(Some(format!("Error: {e}")));
                } else {
                    set_auto_msg(Some("Auto finder started".into()));
                    set_auto_active(true);
                }
            });
        }
    };

    use leptos::wasm_bindgen::JsCast;
    use web_sys::Geolocation;

    #[cfg(not(feature = "ssr"))]
    {
        create_effect(move |_| {
            if let Some(window) = web_sys::window() {
                if let Ok(geolocation) = window.navigator().geolocation() {
                    let success_callback = Closure::<dyn FnMut(web_sys::Position)>::new(
                        move |position: web_sys::Position| {
                            set_latitude(position.coords().latitude());
                            set_longitude(position.coords().longitude());
                            set_address_input(format!(
                                "{}, {}",
                                position.coords().latitude(),
                                position.coords().longitude()
                            ));
                            handle_geocode(());
                        },
                    );

                    let _ =
                        geolocation.get_current_position(success_callback.as_ref().unchecked_ref());

                    success_callback.forget();
                }
            }
        });
    }

    view! {
        <div class="max-w-4xl mx-auto p-4">
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold text-gray-800">NSW Available Drivers Tests</h2>
            </div>

            <div class="mb-6">
                <div class="flex flex-wrap gap-4 items-end">
                    <div class="flex flex-col flex-grow">
                        <label for="address" class="text-sm font-medium text-gray-700 mb-1">
                            Search by Postcode, Address, or Suburb:
                        </label>
                        <input
                            id="address"
                            type="text"
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                            placeholder="e.g., Sydney, 2000, 42 Wallaby Way"
                            prop:value={address_input}
                            on:input=move |ev| set_address_input(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" {
                                    handle_geocode(());
                                }
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500 italic">Your search is securely processed through nominatim.org, a trusted open-source geolocation service. No personal or identifying information is shared during this process.</p>
                    </div>
                </div>

                <div class="flex items-center gap-4 mt-2 w-full">
                    <button
                        class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors"
                        on:click=move |_| handle_geocode(())
                    >
                        Search
                    </button>
                    <button
                        class="px-4 py-2 bg-purple-600 text-white rounded-md hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 transition-colors"
                        on:click=move |_| toggle_auto_panel(())
                    >
                        Auto Test Finder
                    </button>

                    <div class="ml-auto text-sm text-gray-500">
                        {move || match last_updated.get() {
                            Some(time) => view! {
                                <span>"Data last updated: " <TimeDisplay iso_time={time} /></span>
                            }.into_any(),
                            None => view! { <span>"Data last updated: unknown"</span> }.into_any(),
                        }}
                    </div>
                </div>

                <div class="mt-2">
                    {move || {
                        match geocoding_status.get() {
                            Some(status) => view! {
                                <div class="text-sm mt-2 text-amber-600">
                                    {status}
                                </div>
                            }.into_any(),
                            None => view! { <div class="hidden"></div> }.into_any()
                        }
                    }}
                </div>

                <div class="mt-4 flex flex-wrap gap-4 items-end">
                    <div class="flex flex-wrap gap-4">
                        <div class="flex flex-col">
                            <label class="text-sm font-medium text-gray-700 mb-1">Current Coordinates:</label>
                            <div class="text-sm text-gray-600">
                                {move || format!("Lat: {:.6}, Lng: {:.6}", latitude.get(), longitude.get())}
                            </div>
                        </div>

                        <div class="flex flex-col">
                            <label class="text-sm font-medium text-gray-700 mb-1">Location:</label>
                            <div class="text-sm text-gray-600 max-w-md truncate">
                                {move || current_location_name.get()}
                            </div>
                        </div>
                    </div>
                </div>

                <p class="mt-1 text-xs text-gray-500 italic">
                  "Disclaimer: Pass rates shown are calculated based on the "
                  <span class="text-amber-600">center</span> " of the customer's local government
                  area (LGA) and weighted according to proximity to nearby testing centers. "
                  <span class="text-amber-600">These rates are estimates only.</span>
                  " Data is from 2022-2025 C Class Driver tests."
                </p>

                <div class="mt-4 flex flex-wrap gap-4 items-end">
                    <input
                        type="text"
                        class="px-3 py-2 border border-gray-300 rounded-md"
                        placeholder="Booking ID"
                        prop:value={booking_id_input}
                        on:input=move |ev| set_booking_id_input(event_target_value(&ev))
                    />
                    <input
                        type="text"
                        class="px-3 py-2 border border-gray-300 rounded-md"
                        placeholder="Last name"
                        prop:value={last_name_input}
                        on:input=move |ev| set_last_name_input(event_target_value(&ev))
                    />
                    <input
                        type="date"
                        class="px-3 py-2 border border-gray-300 rounded-md"
                        prop:value={latest_date_input}
                        on:input=move |ev| set_latest_date_input(event_target_value(&ev))
                    />
                    <button
                        class="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700"
                        on:click=move |_| handle_find_slot(())
                    >"Go"</button>
                </div>
                <div class="mt-2 text-sm text-emerald-600">
                    {move || match find_slot_msg.get() { Some(ref m) => m.clone(), None => String::new() }}
                </div>


                {move || if show_auto_panel.get() {
                    view! {
                        <div class="mt-4 p-4 border rounded-md w-full">
                            <div class="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
                                {location_manager.get_all().into_iter().map(|loc| {
                                    let name = loc.name.clone();
                                    view! {
                                        <label class="flex items-center gap-1 text-sm">
                                            <input type="checkbox" checked={selected_locations.get().contains(&name)} on:change=move |_| toggle_location(name.clone()) />
                                            {name.clone()}
                                        </label>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                            <div class="mt-2 flex items-center gap-4">
                                <button class="px-4 py-2 bg-purple-600 text-white rounded-md" on:click=move |_| handle_auto_action(())>
                                    {move || if auto_active.get() { "Deactivate" } else { "Activate" }}
                                </button>
                                <span class="text-sm">
                                    <span class={move || if auto_active.get() {"inline-block w-3 h-3 rounded-full bg-green-500"} else {"inline-block w-3 h-3 rounded-full bg-red-500"}}></span>
                                </span>
                            </div>
                            <div class="mt-2 text-sm text-emerald-600">{move || auto_msg.get().unwrap_or_default()}</div>
                        </div>
                    }
                } else { view!{ <div class="hidden"></div> } }
                }

            </div>

            <LocationsTable
                bookings=bookings
                is_loading=is_fetching_bookings
                latitude=latitude
                longitude=longitude
                location_manager=location_manager.clone()
                reset_sort_trigger=reset_sort_trigger
            />

            <div class="mt-6 flex justify-between items-center">
                <div class="text-sm text-gray-500">
                    <p>Location search results are made using "https://nominatim.org/" and are always done on your browser, your location information never touches our servers</p>
                    <p>Note: Distances are calculated using the Haversine formula and represent "as the crow flies" distance.</p>
                    <p>You can support me by giving me a github star</p>
                </div>

                <div class="flex gap-2">
                    <a
                        href="https://github.com/teehee567/nsw-drivers-test"
                        target="_blank"
                        class="px-3 py-1.5 bg-gray-800 text-white rounded-md hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 transition-colors inline-flex items-center justify-center gap-2"
                    >
                        <i class="fab fa-github"></i>
                        <span>View on GitHub</span>
                    </a>
                </div>
            </div>
        </div>
    }
}