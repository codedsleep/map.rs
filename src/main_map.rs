use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Box as GtkBox, HeaderBar, Orientation, Button, Entry, Image, MenuButton, Settings, Switch, Label, Popover};
use webkit2gtk::{WebView, WebViewExt, UserContentManager, UserContentManagerExt, UserScript, UserScriptInjectionTime, UserContentInjectedFrames};
use std::path::Path;
use std::sync::{Arc, Mutex};

mod geolocation;
mod routing;

use geolocation::{GeolocationService, Location};
use routing::{RoutingService, Waypoint};

const APP_ID: &str = "org.example.map-rs";

fn main() -> glib::ExitCode {
    // Initialize Tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();
    
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
        .title("Map.rs")
        .build();
    
    // Add settings menu button to the header bar
    let settings_menu_button = MenuButton::new();
    let settings_icon = Image::from_icon_name(Some("preferences-system"), gtk::IconSize::Button);
    settings_menu_button.set_image(Some(&settings_icon));
    settings_menu_button.set_tooltip_text(Some("Settings"));
    
    // Create settings popover with toggle switches
    let settings_popover = Popover::new(Some(&settings_menu_button));
    
    let popover_box = GtkBox::new(Orientation::Vertical, 10);
    popover_box.set_margin_start(15);
    popover_box.set_margin_end(15);
    popover_box.set_margin_top(10);
    popover_box.set_margin_bottom(10);
    
    // Theme setting
    let theme_row = GtkBox::new(Orientation::Horizontal, 10);
    let theme_label = Label::new(Some("Light Mode"));
    let theme_toggle = Switch::new();
    theme_row.pack_start(&theme_label, false, false, 0);
    theme_row.pack_end(&theme_toggle, false, false, 0);
    
    // Units setting
    let units_row = GtkBox::new(Orientation::Horizontal, 10);
    let units_label = Label::new(Some("Miles"));
    let units_toggle = Switch::new();
    units_toggle.set_active(true); // Default to miles (true = miles, false = km)
    units_row.pack_start(&units_label, false, false, 0);
    units_row.pack_end(&units_toggle, false, false, 0);
    
    popover_box.pack_start(&theme_row, false, false, 0);
    popover_box.pack_start(&units_row, false, false, 0);
    
    settings_popover.add(&popover_box);
    popover_box.show_all();
    
    // Set the popover on the menu button (this is the proper way for MenuButton)
    settings_menu_button.set_popover(Some(&settings_popover));
    
    // Add close button to the right side of the header bar
    let close_button = Button::new();
    let close_icon = Image::from_icon_name(Some("window-close"), gtk::IconSize::Button);
    close_button.set_image(Some(&close_icon));
    close_button.set_tooltip_text(Some("Close application"));
    header_bar.pack_end(&close_button);
    
    // Add settings menu to the left of close button
    header_bar.pack_end(&settings_menu_button);
    
    // Connect theme toggle functionality
    {
        let label_clone = theme_label.clone();
        theme_toggle.connect_state_set(move |_, is_active| {
            if is_active {
                println!("🌙 Switching to dark mode");
                label_clone.set_text("Dark Mode");
                if let Some(settings) = Settings::default() {
                    settings.set_gtk_application_prefer_dark_theme(true);
                    settings.set_gtk_theme_name(Some("Adwaita"));
                }
            } else {
                println!("🌞 Switching to light mode");
                label_clone.set_text("Light Mode");
                if let Some(settings) = Settings::default() {
                    settings.set_gtk_application_prefer_dark_theme(false);
                    settings.set_gtk_theme_name(Some("Adwaita"));
                }
            }
            glib::Propagation::Proceed
        });
    }
    
    // Connect close button to quit the application
    {
        let window_weak = window.downgrade();
        close_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                window.close();
            }
        });
    }
    
    window.set_titlebar(Some(&header_bar));

    // Initialize services and shared state
    let geo_service = Arc::new(Mutex::new(GeolocationService::new()));
    let routing_service = Arc::new(RoutingService::new());
    let use_miles = Arc::new(Mutex::new(true)); // Default to miles
    
    // Connect units toggle functionality
    {
        let label_clone = units_label.clone();
        let use_miles_clone = use_miles.clone();
        units_toggle.connect_state_set(move |_, is_active| {
            if is_active {
                println!("📏 Switching to miles");
                label_clone.set_text("Miles");
                *use_miles_clone.lock().unwrap() = true;
            } else {
                println!("📏 Switching to kilometers");
                label_clone.set_text("Kilometers");
                *use_miles_clone.lock().unwrap() = false;
            }
            glib::Propagation::Proceed
        });
    }
    
    // Main container
    let main_box = GtkBox::new(Orientation::Vertical, 0);
    
    // Controls section
    let controls_box = GtkBox::new(Orientation::Horizontal, 10);
    controls_box.set_margin_start(10);
    controls_box.set_margin_end(10);
    controls_box.set_margin_top(10);
    controls_box.set_margin_bottom(10);
    
    // Location search controls
    let location_entry = Entry::builder()
        .placeholder_text("Enter location (e.g., London, UK)")
        .width_request(300)
        .build();
    
    let search_button = Button::with_label("Search");
    let location_button = Button::with_label("My Location");
    let route_button = Button::with_label("Plan Route");
    let clear_button = Button::with_label("Clear");
    let directions_toggle = Button::with_label("Directions");
    
    controls_box.pack_start(&location_entry, false, false, 0);
    controls_box.pack_start(&search_button, false, false, 0);
    controls_box.pack_start(&location_button, false, false, 0);
    controls_box.pack_start(&route_button, false, false, 0);
    controls_box.pack_start(&directions_toggle, false, false, 0);
    controls_box.pack_start(&clear_button, false, false, 0);
    
    // WebView setup
    let user_content_manager = UserContentManager::new();
    let webview = WebView::with_user_content_manager(&user_content_manager);
    webview.set_vexpand(true);
    webview.set_hexpand(true);
    
    // Directions pane
    let directions_container = GtkBox::new(Orientation::Vertical, 0);
    directions_container.set_width_request(300);
    
    // Add directions title
    let directions_title = Label::new(Some("Directions"));
    directions_title.set_markup("<b>Directions</b>");
    directions_title.set_xalign(0.0);
    directions_title.set_margin_start(10);
    directions_title.set_margin_end(10);
    directions_title.set_margin_top(10);
    directions_title.set_margin_bottom(5);
    
    let directions_scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    directions_scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    
    let directions_box = GtkBox::new(Orientation::Vertical, 5);
    directions_box.set_margin_start(10);
    directions_box.set_margin_end(10);
    directions_box.set_margin_top(5);
    directions_box.set_margin_bottom(10);
    
    let directions_label = Label::new(Some("Click 'Plan Route' to see turn-by-turn directions"));
    directions_label.set_line_wrap(true);
    directions_label.set_xalign(0.0);
    directions_box.pack_start(&directions_label, false, false, 0);
    
    directions_scrolled.add(&directions_box);
    directions_container.pack_start(&directions_title, false, false, 0);
    directions_container.pack_start(&directions_scrolled, true, true, 0);
    
    // Will hide directions pane after show_all()
    
    // Set up WebView with message handlers
    setup_webview(&webview, &user_content_manager, geo_service.clone(), routing_service.clone(), directions_box.clone(), directions_container.clone(), use_miles.clone());
    
    // Load the HTML map
    load_map_html(&webview);
    
    // Content area with map and directions pane
    let content_box = GtkBox::new(Orientation::Horizontal, 0);
    
    content_box.pack_start(&directions_container, false, false, 0);
    content_box.pack_start(&webview, true, true, 0);
    
    main_box.pack_start(&controls_box, false, false, 0);
    main_box.pack_start(&content_box, true, true, 0);
    
    window.add(&main_box);
    
    // Set up event handlers
    setup_event_handlers(
        geo_service,
        routing_service,
        location_entry,
        search_button,
        location_button,
        route_button,
        clear_button,
        directions_toggle,
        webview.clone(),
        directions_box.clone(),
        directions_container.clone(),
        use_miles.clone(),
    );
    
    // Add Escape key handler to clear map
    {
        let webview = webview.clone();
        let directions_box = directions_box.clone();
        let directions_container = directions_container.clone();
        window.connect_key_press_event(move |_, event_key| {
            if event_key.keyval() == gtk::gdk::keys::constants::Escape {
                println!("🧹 Escape pressed - clearing map...");
                let js_code = "if (window.clearMap) { window.clearMap(); }";
                webview.evaluate_javascript(
                    js_code,
                    None,
                    None,
                    webkit2gtk::gio::Cancellable::NONE,
                    |_| {}
                );
                
                // Clear directions pane and hide it
                let children: Vec<gtk::Widget> = directions_box.children();
                for child in children {
                    directions_box.remove(&child);
                }
                
                let directions_label = Label::new(Some("Click 'Plan Route' to see turn-by-turn directions"));
                directions_label.set_line_wrap(true);
                directions_label.set_xalign(0.0);
                directions_box.pack_start(&directions_label, false, false, 0);
                directions_box.show_all();
                
                // Hide directions pane
                directions_container.set_visible(false);
                
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    
    window.show_all();
    
    // Hide directions pane by default (after show_all)
    directions_container.set_visible(false);
}

fn setup_webview(
    webview: &WebView,
    user_content_manager: &UserContentManager,
    geo_service: Arc<Mutex<GeolocationService>>,
    routing_service: Arc<RoutingService>,
    directions_box: GtkBox,
    directions_container: GtkBox,
    use_miles: Arc<Mutex<bool>>,
) {
    // Inject JavaScript for Rust communication
    let init_script = UserScript::new(
        r#"
        window.rustBackend = {
            sendMessage: function(type, data) {
                if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.rustHandler) {
                    window.webkit.messageHandlers.rustHandler.postMessage({
                        type: type,
                        data: data
                    });
                }
            },
            
            onLocationClick: function(lat, lng) {
                this.sendMessage('location_click', {lat: lat, lng: lng});
            },
            
            onLocationUpdate: function(location) {
                this.sendMessage('location_update', location);
            }
        };
        
        console.log('✅ Rust backend bridge ready');
        "#,
        UserContentInjectedFrames::AllFrames,
        UserScriptInjectionTime::Start,
        &[],
        &[],
    );
    
    user_content_manager.add_script(&init_script);
    
    // Register JS-to-Rust message handler
    user_content_manager.register_script_message_handler("rustHandler");
    
    let routing_service_clone = routing_service.clone();
    let webview_clone = webview.clone();
    let directions_box_clone = directions_box.clone();
    let directions_container_clone = directions_container.clone();
    let use_miles_clone = use_miles.clone();
    
    user_content_manager.connect_script_message_received(Some("rustHandler"), move |_, msg: &webkit2gtk::JavascriptResult| {
        // Convert to string and try to parse as JSON
        let js_string = msg.js_value().map(|v| v.to_string()).unwrap_or_default();
        println!("Received message string from JS: '{}'", js_string);
        
        // Try to parse as JSON directly
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&js_string) {
            if let Some(msg_type) = parsed.get("type").and_then(|v| v.as_str()) {
                println!("Message type: {}", msg_type);
                match msg_type {
                    "calculate_route" => {
                        if let Some(waypoints_json) = parsed.get("waypoints") {
                            let routing_service = routing_service_clone.clone();
                            let webview = webview_clone.clone();
                            let directions_box = directions_box_clone.clone();
                            let directions_container = directions_container_clone.clone();
                            let use_miles = use_miles_clone.clone();
                            
                            println!("Parsing waypoints: {:?}", waypoints_json);
                            
                            // Parse waypoints
                            if let Ok(waypoints_data) = serde_json::from_value::<Vec<serde_json::Value>>(waypoints_json.clone()) {
                                let waypoints: Vec<Waypoint> = waypoints_data
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(i, wp)| {
                                        if let (Some(lat), Some(lng)) = (
                                            wp.get("lat").and_then(|v| v.as_f64()),
                                            wp.get("lng").and_then(|v| v.as_f64())
                                        ) {
                                            Some(Waypoint {
                                                lat,
                                                lng,
                                                name: Some(format!("Point {}", i + 1)),
                                            })
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                
                                println!("Parsed {} waypoints", waypoints.len());
                                
                                if waypoints.len() >= 2 {
                                    println!("🛣️ Calculating route for {} waypoints", waypoints.len());
                                    
                                    glib::spawn_future_local(async move {
                                        let use_miles_val = *use_miles.lock().unwrap();
                                        match routing_service.calculate_route(&waypoints, use_miles_val).await {
                                            Ok(route) => {
                                                let distance_text = if use_miles_val {
                                                    let miles = route.distance * 0.000621371;
                                                    format!("{:.1} mi", miles)
                                                } else {
                                                    format!("{:.1} km", route.distance / 1000.0)
                                                };
                                                
                                                println!("✅ Route: {}, {:.0}min", 
                                                       distance_text, route.duration / 60.0);
                                                
                                                // Update directions UI on the main thread
                                                let route_clone = route.clone();
                                                let directions_box_weak = directions_box.downgrade();
                                                let directions_container_weak = directions_container.downgrade();
                                                let duration_minutes = route.duration / 60.0;
                                                let hours = (duration_minutes / 60.0) as u32;
                                                let minutes = (duration_minutes % 60.0) as u32;
                                                let time_text = if hours > 0 {
                                                    format!("{} hr {} min", hours, minutes)
                                                } else {
                                                    format!("{} min", minutes)
                                                };
                                                let summary_text = format!("Route: {}, {}", distance_text, time_text);
                                                glib::idle_add_local_once(move || {
                                                    // Auto-show directions pane when route is calculated
                                                    if let Some(directions_container) = directions_container_weak.upgrade() {
                                                        directions_container.set_visible(true);
                                                    }
                                                    
                                                    if let Some(directions_box) = directions_box_weak.upgrade() {
                                                        // Clear existing directions
                                                        let children: Vec<gtk::Widget> = directions_box.children();
                                                        for child in children {
                                                            directions_box.remove(&child);
                                                        }
                                                        
                                                        // Add route summary
                                                        let summary_label = Label::new(Some(&summary_text));
                                                        summary_label.set_line_wrap(true);
                                                        summary_label.set_xalign(0.0);
                                                        summary_label.set_markup(&format!("<b>{}</b>", summary_text));
                                                        directions_box.pack_start(&summary_label, false, false, 0);
                                                        
                                                        // Add separator
                                                        let separator = gtk::Separator::new(Orientation::Horizontal);
                                                        directions_box.pack_start(&separator, false, false, 5);
                                                        
                                                        // Add turn-by-turn directions
                                                        for (i, instruction) in route_clone.instructions.iter().enumerate() {
                                                            let direction_label = Label::new(Some(&format!(
                                                                "{}. {}",
                                                                i + 1,
                                                                instruction.text
                                                            )));
                                                            direction_label.set_line_wrap(true);
                                                            direction_label.set_xalign(0.0);
                                                            direction_label.set_margin_bottom(5);
                                                            directions_box.pack_start(&direction_label, false, false, 0);
                                                        }
                                                        
                                                        directions_box.show_all();
                                                    }
                                                });
                                                
                                                // Send route to map
                                                let js_code = format!(
                                                    "if (window.mapInstance && window.addRouteToMap) {{ \
                                                        window.addRouteToMap('{}'); \
                                                    }}",
                                                    route.geometry.replace("'", "\\'")
                                                );
                                                
                                                webview.evaluate_javascript(
                                                    &js_code,
                                                    None,
                                                    None,
                                                    webkit2gtk::gio::Cancellable::NONE,
                                                    |_| {}
                                                );
                                            }
                                            Err(e) => {
                                                println!("❌ Route error: {}", e);
                                                let js_code = format!("alert('Route calculation failed: {}');", e);
                                                webview.evaluate_javascript(
                                                    &js_code,
                                                    None,
                                                    None,
                                                    webkit2gtk::gio::Cancellable::NONE,
                                                    |_| {}
                                                );
                                            }
                                        }
                                    });
                                } else {
                                    println!("❌ Need at least 2 waypoints, got {}", waypoints.len());
                                }
                            } else {
                                println!("❌ Failed to parse waypoints JSON");
                            }
                        } else {
                            println!("❌ No waypoints found in message");
                        }
                    }
                    _ => {
                        println!("Unknown message type: {}", msg_type);
                    }
                }
            } else {
                println!("❌ No message type found in JSON");
            }
        } else {
            println!("❌ Failed to parse JSON: '{}'", js_string);
        }
    });

    println!("📡 WebView communication bridge initialized");
}

// Message handling would be implemented here in a full version

fn load_map_html(webview: &WebView) {
    let html_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("map.html");
    
    if html_path.exists() {
        let file_uri = format!("file://{}", html_path.display());
        webview.load_uri(&file_uri);
    } else {
        // Enhanced fallback with full Leaflet functionality
        let html_content = include_str!("map_embedded.html");
        webview.load_html(html_content, None);
    }
}

fn setup_event_handlers(
    geo_service: Arc<Mutex<GeolocationService>>,
    routing_service: Arc<RoutingService>,
    location_entry: Entry,
    search_button: Button,
    location_button: Button,
    route_button: Button,
    clear_button: Button,
    directions_toggle: Button,
    webview: WebView,
    directions_box: GtkBox,
    directions_container: GtkBox,
    use_miles: Arc<Mutex<bool>>,
) {
    // Search location handler
    {
        let routing_service = routing_service.clone();
        let location_entry = location_entry.clone();
        let webview = webview.clone();
        
        search_button.connect_clicked(move |_| {
            let query = location_entry.text().to_string();
            if query.is_empty() {
                return;
            }
            
            println!("🔍 Searching for: {}", query);
            
            let routing_service = routing_service.clone();
            let webview = webview.clone();
            
            glib::spawn_future_local(async move {
                match routing_service.geocode(&query).await {
                    Ok(locations) => {
                        if let Some(location) = locations.first() {
                            println!("📍 Found: {:.6}, {:.6}", location.latitude, location.longitude);
                            
                            // Send to map
                            let js_code = format!(
                                "if (window.mapInstance) {{ \
                                    window.mapInstance.setView([{}, {}], 15); \
                                    var marker = L.marker([{}, {}]).addTo(window.mapInstance) \
                                        .bindPopup('{}').openPopup(); \
                                    if (!window.clickMarkers) window.clickMarkers = []; \
                                    window.clickMarkers.push(marker); \
                                }}",
                                location.latitude, location.longitude,
                                location.latitude, location.longitude,
                                query.replace("'", "\\'")
                            );
                            
                            webview.evaluate_javascript(
                                &js_code,
                                None,
                                None,
                                webkit2gtk::gio::Cancellable::NONE,
                                |_| {}
                            );
                        }
                    }
                    Err(e) => {
                        println!("❌ Search error: {}", e);
                    }
                }
            });
        });
    }
    
    // Enter key handler for search
    {
        let routing_service = routing_service.clone();
        let location_entry = location_entry.clone();
        let webview = webview.clone();
        
        location_entry.connect_activate(move |entry| {
            let query = entry.text().to_string();
            if query.is_empty() {
                return;
            }
            
            println!("🔍 Searching for: {}", query);
            
            let routing_service = routing_service.clone();
            let webview = webview.clone();
            
            glib::spawn_future_local(async move {
                match routing_service.geocode(&query).await {
                    Ok(locations) => {
                        if let Some(location) = locations.first() {
                            println!("📍 Found: {:.6}, {:.6}", location.latitude, location.longitude);
                            
                            // Send to map
                            let js_code = format!(
                                "if (window.mapInstance) {{ \
                                    window.mapInstance.setView([{}, {}], 15); \
                                    var marker = L.marker([{}, {}]).addTo(window.mapInstance) \
                                        .bindPopup('{}').openPopup(); \
                                    if (!window.clickMarkers) window.clickMarkers = []; \
                                    window.clickMarkers.push(marker); \
                                }}",
                                location.latitude, location.longitude,
                                location.latitude, location.longitude,
                                query.replace("'", "\\'")
                            );
                            
                            webview.evaluate_javascript(
                                &js_code,
                                None,
                                None,
                                webkit2gtk::gio::Cancellable::NONE,
                                |_| {}
                            );
                        }
                    }
                    Err(e) => {
                        println!("❌ Search error: {}", e);
                    }
                }
            });
        });
    }
    
    // Current location handler
    {
        let geo_service = geo_service.clone();
        let webview = webview.clone();
        
        location_button.connect_clicked(move |_| {
            println!("📍 Getting current location...");
            
            // Simulate getting location (London)
            let location = Location::new(51.5074, -0.1278).with_accuracy(10.0);
            
            {
                let mut service = geo_service.lock().unwrap();
                service.update_location(location.clone());
            }
            
            println!("✅ Location: {:.6}, {:.6}", location.latitude, location.longitude);
            
            // Send to map
            let js_code = format!(
                "if (window.mapInstance) {{ \
                    window.mapInstance.setView([{}, {}], 15); \
                    if (window.currentLocationMarker) {{ \
                        window.mapInstance.removeLayer(window.currentLocationMarker); \
                    }} \
                    var marker = L.marker([{}, {}]).addTo(window.mapInstance) \
                        .bindPopup('You are here!').openPopup(); \
                    window.currentLocationMarker = marker; \
                    if (!window.clickMarkers) window.clickMarkers = []; \
                    window.clickMarkers.push(marker); \
                }}",
                location.latitude, location.longitude,
                location.latitude, location.longitude
            );
            
            webview.evaluate_javascript(
                &js_code,
                None,
                None,
                webkit2gtk::gio::Cancellable::NONE,
                |_| {}
            );
        });
    }
    
    // Route planning handler - uses clicked markers as waypoints
    {
        let routing_service = routing_service.clone();
        let webview = webview.clone();
        
        route_button.connect_clicked(move |_| {
            println!("🛣️ Planning route with clicked markers...");
            
            // Get waypoints from the map by evaluating JavaScript
            let routing_service = routing_service.clone();
            let webview = webview.clone();
            
            let js_code = r#"
                console.log('Route button clicked');
                console.log('clickMarkers:', window.clickMarkers);
                console.log('clickMarkers length:', window.clickMarkers ? window.clickMarkers.length : 'undefined');
                
                if (window.clickMarkers && window.clickMarkers.length >= 2) {
                    var waypoints = window.clickMarkers.map(function(marker) {
                        var latlng = marker.getLatLng();
                        return {lat: latlng.lat, lng: latlng.lng};
                    });
                    console.log('Sending waypoints:', waypoints);
                    
                    var message = {
                        type: 'calculate_route',
                        waypoints: waypoints
                    };
                    console.log('Sending message:', message);
                    
                    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.rustHandler) {
                        // Convert message to JSON string before sending
                        var jsonMessage = JSON.stringify(message);
                        console.log('Sending JSON string:', jsonMessage);
                        window.webkit.messageHandlers.rustHandler.postMessage(jsonMessage);
                        console.log('Message sent to Rust');
                    } else {
                        console.error('Rust message handler not available');
                        alert('Rust backend not connected');
                    }
                } else {
                    console.log('Not enough markers for route');
                    alert('Please click at least 2 points on the map first to create a route.');
                }
            "#;
            
            webview.evaluate_javascript(
                js_code,
                None,
                None,
                webkit2gtk::gio::Cancellable::NONE,
                |_| {}
            );
        });
    }
    
    // Directions toggle handler
    {
        let directions_container = directions_container.clone();
        directions_toggle.connect_clicked(move |_button| {
            let is_visible = directions_container.is_visible();
            directions_container.set_visible(!is_visible);
            
            if is_visible {
                println!("📋 Hiding directions pane");
            } else {
                println!("📋 Showing directions pane");
            }
        });
    }
    
    // Clear map handler
    {
        let webview = webview.clone();
        let directions_box = directions_box.clone();
        let directions_container = directions_container.clone();
        
        clear_button.connect_clicked(move |_| {
            let js_code = "if (window.clearMap) { window.clearMap(); }";
            webview.evaluate_javascript(
                js_code,
                None,
                None,
                webkit2gtk::gio::Cancellable::NONE,
                |_| {}
            );
            
            // Clear directions pane and hide it
            let children: Vec<gtk::Widget> = directions_box.children();
            for child in children {
                directions_box.remove(&child);
            }
            
            let directions_label = Label::new(Some("Click 'Plan Route' to see turn-by-turn directions"));
            directions_label.set_line_wrap(true);
            directions_label.set_xalign(0.0);
            directions_box.pack_start(&directions_label, false, false, 0);
            directions_box.show_all();
            
            // Hide directions pane
            directions_container.set_visible(false);
        });
    }
}