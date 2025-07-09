use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: Option<f64>,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationUpdate {
    pub lat: f64,
    pub lng: f64,
    pub accuracy: Option<f64>,
}

impl From<LocationUpdate> for Location {
    fn from(update: LocationUpdate) -> Self {
        let mut location = Location::new(update.lat, update.lng);
        if let Some(accuracy) = update.accuracy {
            location = location.with_accuracy(accuracy);
        }
        location
    }
}

impl Location {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            accuracy: None,
            timestamp: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()),
        }
    }

    pub fn with_accuracy(mut self, accuracy: f64) -> Self {
        self.accuracy = Some(accuracy);
        self
    }

    pub fn distance_to(&self, other: &Location) -> f64 {
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        
        6371000.0 * c // Earth radius in meters
    }
}

pub struct GeolocationService {
    current_location: Option<Location>,
    location_history: Vec<Location>,
}

impl GeolocationService {
    pub fn new() -> Self {
        Self {
            current_location: None,
            location_history: Vec::new(),
        }
    }

    pub fn update_location(&mut self, location: Location) {
        self.location_history.push(location.clone());
        self.current_location = Some(location);
        
        // Keep only last 100 locations to manage memory
        if self.location_history.len() > 100 {
            self.location_history.remove(0);
        }
    }

    pub fn get_current_location(&self) -> Option<&Location> {
        self.current_location.as_ref()
    }

    pub fn get_location_history(&self) -> &[Location] {
        &self.location_history
    }
}

impl Default for GeolocationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_creation() {
        let loc = Location::new(51.505, -0.09);
        assert_eq!(loc.latitude, 51.505);
        assert_eq!(loc.longitude, -0.09);
        assert!(loc.timestamp.is_some());
    }

    #[test]
    fn test_location_distance() {
        let london = Location::new(51.5074, -0.1278);
        let paris = Location::new(48.8566, 2.3522);
        
        let distance = london.distance_to(&paris);
        assert!(distance > 300000.0); // Should be > 300km
        assert!(distance < 400000.0); // Should be < 400km
    }

    #[test]
    fn test_geolocation_service() {
        let mut service = GeolocationService::new();
        assert!(service.get_current_location().is_none());
        
        let location = Location::new(51.505, -0.09);
        service.update_location(location.clone());
        
        assert!(service.get_current_location().is_some());
        assert_eq!(service.get_location_history().len(), 1);
    }
}