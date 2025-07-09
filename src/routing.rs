use serde::{Deserialize, Serialize};
use crate::geolocation::Location;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    pub lat: f64,
    pub lng: f64,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequest {
    pub waypoints: Vec<Waypoint>,
    pub profile: String, // "driving", "walking", "cycling"
}

impl Default for RouteRequest {
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            profile: "driving".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResponse {
    pub distance: f64, // in meters
    pub duration: f64, // in seconds  
    pub geometry: String, // encoded polyline or GeoJSON
    pub instructions: Vec<RouteInstruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInstruction {
    pub text: String,
    pub distance: f64,
    pub duration: f64,
    pub location: Location,
}

pub struct RoutingService {
    pub osm_api_base: String,
}

impl RoutingService {
    pub fn new() -> Self {
        Self {
            osm_api_base: "https://router.project-osrm.org".to_string(),
        }
    }

    pub async fn calculate_route(&self, waypoints: &[Waypoint], use_miles: bool) -> Result<RouteResponse, Box<dyn std::error::Error>> {
        if waypoints.len() < 2 {
            return Err("At least 2 waypoints are required".into());
        }

        // Build coordinates string for OSRM API
        let coordinates: Vec<String> = waypoints
            .iter()
            .map(|wp| format!("{},{}", wp.lng, wp.lat))
            .collect();
        
        let coordinates_str = coordinates.join(";");
        
        // Use OSRM API for routing with enhanced parameters for better instructions
        let url = format!(
            "{}/route/v1/driving/{}?overview=full&geometries=geojson&steps=true&annotations=true&continue_straight=true",
            self.osm_api_base, coordinates_str
        );

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("Routing API error: {}", response.status()).into());
        }

        let osrm_response: OSRMResponse = response.json().await?;
        
        if osrm_response.routes.is_empty() {
            return Err("No route found".into());
        }

        let route = &osrm_response.routes[0];
        let instructions = self.parse_instructions(&route.legs, use_miles);

        Ok(RouteResponse {
            distance: route.distance,
            duration: route.duration,
            geometry: serde_json::to_string(&route.geometry)?,
            instructions,
        })
    }

    fn parse_instructions(&self, legs: &[OSRMLeg], use_miles: bool) -> Vec<RouteInstruction> {
        let mut instructions = Vec::new();
        
        for leg in legs {
            for step in &leg.steps {
                let instruction_text = self.generate_instruction_text(step, use_miles);
                
                instructions.push(RouteInstruction {
                    text: instruction_text,
                    distance: step.distance,
                    duration: step.duration,
                    location: Location::new(
                        step.maneuver.location[1],
                        step.maneuver.location[0],
                    ),
                });
            }
        }
        
        instructions
    }
    
    fn generate_instruction_text(&self, step: &OSRMStep, use_miles: bool) -> String {
        let maneuver_type = step.maneuver.maneuver_type.as_deref().unwrap_or("continue");
        let modifier = step.maneuver.modifier.as_deref();
        let road_name = step.name.as_deref().unwrap_or("");
        let road_ref = step.ref_.as_deref();
        
        // Format distance in a more readable way
        let distance_text = self.format_distance(step.distance, use_miles);
        
        // Build the street name part
        let street_info = if !road_name.is_empty() {
            if let Some(ref_) = road_ref {
                format!("on {} ({})", road_name, ref_)
            } else {
                format!("on {}", road_name)
            }
        } else if let Some(ref_) = road_ref {
            format!("on {}", ref_)
        } else {
            String::new()
        };
        
        // Generate instruction based on maneuver type
        match maneuver_type {
            "depart" => {
                if street_info.is_empty() {
                    format!("Head {} for {}", 
                           self.bearing_to_direction(step.maneuver.bearing_after), 
                           distance_text)
                } else {
                    format!("Head {} {} for {}", 
                           self.bearing_to_direction(step.maneuver.bearing_after), 
                           street_info, 
                           distance_text)
                }
            }
            "turn" => {
                let direction = modifier.unwrap_or("").replace("sharp ", "sharp ");
                if street_info.is_empty() {
                    format!("Turn {} for {}", direction, distance_text)
                } else {
                    format!("Turn {} {} for {}", direction, street_info, distance_text)
                }
            }
            "merge" => {
                let direction = modifier.unwrap_or("").replace("slight ", "");
                if street_info.is_empty() {
                    format!("Merge {} for {}", direction, distance_text)
                } else {
                    format!("Merge {} {} for {}", direction, street_info, distance_text)
                }
            }
            "ramp" => {
                let direction = modifier.unwrap_or("").replace("slight ", "");
                if street_info.is_empty() {
                    format!("Take the ramp {} for {}", direction, distance_text)
                } else {
                    format!("Take the ramp {} {} for {}", direction, street_info, distance_text)
                }
            }
            "fork" => {
                let direction = modifier.unwrap_or("left");
                if street_info.is_empty() {
                    format!("Keep {} at the fork for {}", direction, distance_text)
                } else {
                    format!("Keep {} at the fork {} for {}", direction, street_info, distance_text)
                }
            }
            "roundabout" => {
                if street_info.is_empty() {
                    format!("Enter the roundabout for {}", distance_text)
                } else {
                    format!("Enter the roundabout and take {} for {}", street_info, distance_text)
                }
            }
            "arrive" => {
                "Arrive at your destination".to_string()
            }
            _ => {
                // Default case for "continue" and other types
                if street_info.is_empty() {
                    format!("Continue for {}", distance_text)
                } else {
                    format!("Continue {} for {}", street_info, distance_text)
                }
            }
        }
    }
    
    fn format_distance(&self, meters: f64, use_miles: bool) -> String {
        if use_miles {
            let miles = meters * 0.000621371; // Convert meters to miles
            format!("{:.1} mi", miles)
        } else {
            if meters >= 1000.0 {
                format!("{:.1} km", meters / 1000.0)
            } else {
                format!("{:.0} m", meters)
            }
        }
    }
    
    fn bearing_to_direction(&self, bearing: Option<f64>) -> String {
        match bearing {
            Some(b) => {
                let normalized = ((b % 360.0) + 360.0) % 360.0;
                match normalized {
                    b if b < 22.5 || b >= 337.5 => "north",
                    b if b < 67.5 => "northeast", 
                    b if b < 112.5 => "east",
                    b if b < 157.5 => "southeast",
                    b if b < 202.5 => "south",
                    b if b < 247.5 => "southwest",
                    b if b < 292.5 => "west",
                    _ => "northwest",
                }
            }
            None => "straight",
        }.to_string()
    }

    pub async fn geocode(&self, query: &str) -> Result<Vec<Location>, Box<dyn std::error::Error>> {
        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "https://nominatim.openstreetmap.org/search?format=json&q={}",
            encoded_query
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", "OSM-Map-App/1.0")
            .send()
            .await?;

        let results: Vec<NominatimResult> = response.json().await?;
        
        Ok(results
            .into_iter()
            .map(|result| Location::new(result.lat.parse().unwrap(), result.lon.parse().unwrap()))
            .collect())
    }
}

impl Default for RoutingService {
    fn default() -> Self {
        Self::new()
    }
}

// OSRM API response structures
#[derive(Debug, Deserialize)]
struct OSRMResponse {
    routes: Vec<OSRMRoute>,
}

#[derive(Debug, Deserialize)]
struct OSRMRoute {
    distance: f64,
    duration: f64,
    geometry: geojson::Geometry,
    legs: Vec<OSRMLeg>,
}

#[derive(Debug, Deserialize)]
struct OSRMLeg {
    distance: f64,
    duration: f64,
    steps: Vec<OSRMStep>,
}

#[derive(Debug, Deserialize)]
struct OSRMStep {
    distance: f64,
    duration: f64,
    maneuver: OSRMManeuver,
    name: Option<String>,
    ref_: Option<String>,
    #[serde(rename = "destinations")]
    destinations: Option<String>,
    mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OSRMManeuver {
    location: [f64; 2],
    instruction: Option<String>,
    #[serde(rename = "type")]
    maneuver_type: Option<String>,
    modifier: Option<String>,
    bearing_after: Option<f64>,
    bearing_before: Option<f64>,
}

// Nominatim API response structure
#[derive(Debug, Deserialize)]
struct NominatimResult {
    lat: String,
    lon: String,
    display_name: String,
}