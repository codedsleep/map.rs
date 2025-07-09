# Map.rs

A desktop map application built with GTK4, Rust, and OpenStreetMap.

![Screenshot From 2025-07-09 07-12-31resize](https://github.com/user-attachments/assets/00de6a1e-b322-42f7-b152-621b286a0b42)

## Features

- Interactive map using Leaflet and OpenStreetMap tiles
- Geolocation support
- Route planning with OSRM API
- Location search using Nominatim
- Memory-safe Rust backend with GTK4 UI

## Prerequisites

### System Dependencies

#### Ubuntu/Debian:
```bash
sudo apt update
sudo apt install libgtk-4-dev libwebkit2gtk-4.0-dev libssl-dev pkg-config build-essential
```

#### Fedora:
```bash
sudo dnf install gtk4-devel webkit2gtk4.0-devel openssl-devel pkg-config gcc
```

#### Arch Linux:
```bash
sudo pacman -S gtk4 webkit2gtk openssl pkg-config base-devel
```

### Rust

Install Rust using rustup:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installation

### AppImage (Recommended)

Download the latest AppImage from the [releases section](https://github.com/your-username/map.rs/releases) and make it executable:

```bash
chmod +x map.rs-*.AppImage
./map.rs-*.AppImage
```

### Building from Source

1. Clone or create the project
2. Install system dependencies (see above)
3. Build and run:

```bash
cargo run
```

## Usage

- **My Location**: Click to center map on your current location
- **Search**: Search for locations using OpenStreetMap's Nominatim service
- **Route**: Click multiple points on the map and then click Route to plan a route
- **Map Interaction**: Click anywhere on the map to see coordinates

## Architecture

- **Frontend**: HTML/CSS/JavaScript with Leaflet for map rendering
- **Backend**: Rust with GTK4 for the native window and webkit2gtk for web content
- **APIs**: 
  - OpenStreetMap tiles for map data
  - OSRM for routing
  - Nominatim for geocoding

## Development

The project is structured as follows:

- `src/main.rs` - Main application and GTK4 setup
- `src/geolocation.rs` - Geolocation services and data structures
- `src/routing.rs` - Route planning and API integration
- `src/map.html` - Frontend map interface

## License

This project uses OpenStreetMap data, which is available under the Open Database License.
