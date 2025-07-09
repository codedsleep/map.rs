use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow, Box, HeaderBar, Orientation, Button, Label, Entry, TextView, TextBuffer, ScrolledWindow};
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::rc::Rc;

mod geolocation;
mod routing;

use geolocation::{GeolocationService, Location};
use routing::{RoutingService, Waypoint};

const APP_ID: &str = "org.example.map-rs";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Map.rs")
        .default_width(1200)
        .default_height(800)
        .build();

    let header_bar = HeaderBar::builder()
        .title_widget(&Label::new(Some("Map.rs")))
        .build();
    
    window.set_titlebar(Some(&header_bar));

    // Initialize services
    let geo_service = Arc::new(Mutex::new(GeolocationService::new()));
    let routing_service = Arc::new(RoutingService::new());
    
    // Main container
    let main_box = Box::new(Orientation::Vertical, 10);
    main_box.set_margin_start(10);
    main_box.set_margin_end(10);
    main_box.set_margin_top(10);
    main_box.set_margin_bottom(10);

    // Controls section
    let controls_box = Box::new(Orientation::Horizontal, 10);
    
    // Location input
    let location_entry = Entry::builder()
        .placeholder_text("Enter location (e.g., London, UK)")
        .build();
    
    let search_button = Button::with_label("Search Location");
    let location_button = Button::with_label("Get Current Location");
    let route_button = Button::with_label("Plan Route");
    
    controls_box.append(&location_entry);
    controls_box.append(&search_button);
    controls_box.append(&location_button);
    controls_box.append(&route_button);
    
    // Map placeholder (in a real app, this would be webkit2gtk)
    let map_placeholder = Label::builder()
        .label("🗺️ Interactive Map Would Appear Here\n\nThis demo shows the Rust backend functionality.\nIn a full implementation, this would be replaced with:\n• webkit2gtk WebView\n• Leaflet + OpenStreetMap tiles\n• Interactive JavaScript controls")
        .justify(gtk4::Justification::Center)
        .css_classes(["map-placeholder"])
        .build();
    
    let map_frame = gtk4::Frame::new(Some("Map Area"));
    map_frame.set_child(Some(&map_placeholder));
    map_frame.set_vexpand(true);
    
    // Output area
    let output_buffer = TextBuffer::new(None);
    let output_view = TextView::with_buffer(&output_buffer);
    output_view.set_editable(false);
    output_view.set_cursor_visible(false);
    
    let output_scroll = ScrolledWindow::builder()
        .height_request(200)
        .child(&output_view)
        .build();
    
    let output_frame = gtk4::Frame::new(Some("Backend Output"));
    output_frame.set_child(Some(&output_scroll));
    
    // Add all components to main box
    main_box.append(&controls_box);
    main_box.append(&map_frame);
    main_box.append(&output_frame);
    
    window.set_child(Some(&main_box));
    
    // Set up event handlers
    setup_event_handlers(
        geo_service,
        routing_service,
        location_entry,
        search_button,
        location_button,
        route_button,
        output_buffer,
    );
    
    // Add CSS styling
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data(
        ".map-placeholder {
            background: #e8f4fd;
            border: 2px dashed #4a90a4;
            padding: 50px;
            font-size: 16px;
            color: #2c3e50;
        }"
    );
    
    gtk4::style_context_add_provider_for_display(
        &gtk4::prelude::WidgetExt::display(&window),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    
    window.present();
}

fn setup_event_handlers(
    geo_service: Arc<Mutex<GeolocationService>>,
    routing_service: Arc<RoutingService>,
    location_entry: Entry,
    search_button: Button,
    location_button: Button,
    route_button: Button,
    output_buffer: TextBuffer,
) {
    let output_buffer = Rc::new(RefCell::new(output_buffer));
    
    // Helper function to add text to output
    let add_output = {
        let output_buffer = output_buffer.clone();
        move |text: &str| {
            let buffer = output_buffer.borrow();
            let mut end_iter = buffer.end_iter();
            buffer.insert(&mut end_iter, &format!("{}\n", text));
        }
    };
    
    // Search location handler
    {
        let routing_service = routing_service.clone();
        let location_entry = location_entry.clone();
        let add_output = add_output.clone();
        
        search_button.connect_clicked(move |_| {
            let query = location_entry.text().to_string();
            if query.is_empty() {
                add_output("Please enter a location to search");
                return;
            }
            
            add_output(&format!("🔍 Searching for: {}", query));
            
            // Spawn async geocoding task
            let routing_service = routing_service.clone();
            let add_output = add_output.clone();
            
            glib::spawn_future_local(async move {
                match routing_service.geocode(&query).await {
                    Ok(locations) => {
                        if locations.is_empty() {
                            add_output("❌ No locations found");
                        } else {
                            for (i, location) in locations.iter().take(3).enumerate() {
                                add_output(&format!(
                                    "📍 Result {}: {:.6}, {:.6}",
                                    i + 1, location.latitude, location.longitude
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        add_output(&format!("❌ Search error: {}", e));
                    }
                }
            });
        });
    }
    
    // Current location handler (simulated)
    {
        let geo_service = geo_service.clone();
        let add_output = add_output.clone();
        
        location_button.connect_clicked(move |_| {
            add_output("📍 Getting current location...");
            
            // Simulate getting location (in real app, would use actual geolocation API)
            let location = Location::new(51.5074, -0.1278).with_accuracy(10.0); // London
            
            {
                let mut service = geo_service.lock().unwrap();
                service.update_location(location.clone());
            }
            
            add_output(&format!(
                "✅ Location found: {:.6}, {:.6} (±{}m)",
                location.latitude, location.longitude, 
                location.accuracy.unwrap_or(0.0)
            ));
        });
    }
    
    // Route planning handler
    {
        let routing_service = routing_service.clone();
        let geo_service = geo_service.clone();
        let add_output = add_output.clone();
        
        route_button.connect_clicked(move |_| {
            let current_location = {
                let service = geo_service.lock().unwrap();
                service.get_current_location().cloned()
            };
            
            if current_location.is_none() {
                add_output("❌ Please get current location first");
                return;
            }
            
            let current = current_location.unwrap();
            
            // Create sample destination (Big Ben)
            let destination = Location::new(51.4994, -0.1245);
            
            let waypoints = vec![
                Waypoint {
                    lat: current.latitude,
                    lng: current.longitude,
                    name: Some("Current Location".to_string()),
                },
                Waypoint {
                    lat: destination.latitude,
                    lng: destination.longitude,
                    name: Some("Big Ben, London".to_string()),
                },
            ];
            
            add_output("🛣️ Planning route...");
            
            let routing_service = routing_service.clone();
            let add_output = add_output.clone();
            
            glib::spawn_future_local(async move {
                match routing_service.calculate_route(&waypoints).await {
                    Ok(route) => {
                        add_output(&format!(
                            "✅ Route found:\n   📏 Distance: {:.1} km\n   ⏱️ Duration: {:.0} minutes\n   📋 {} instructions",
                            route.distance / 1000.0,
                            route.duration / 60.0,
                            route.instructions.len()
                        ));
                        
                        // Show first few instructions
                        for (i, instruction) in route.instructions.iter().take(3).enumerate() {
                            add_output(&format!("   {}. {}", i + 1, instruction.text));
                        }
                        
                        if route.instructions.len() > 3 {
                            add_output(&format!("   ... and {} more steps", route.instructions.len() - 3));
                        }
                    }
                    Err(e) => {
                        add_output(&format!("❌ Route planning error: {}", e));
                    }
                }
            });
        });
    }
    
    // Initial message
    add_output("🚀 OSM Map App Backend Ready!");
    add_output("• Click 'Search Location' to geocode an address");
    add_output("• Click 'Get Current Location' to simulate location detection");
    add_output("• Click 'Plan Route' to calculate a route to Big Ben");
}