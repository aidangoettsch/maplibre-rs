use crate::{coords::WorldTileCoords, style::source::TileAddressingScheme};
use crate::coords::ZoomLevel;

/// Represents a source from which the vector tile are fetched.
#[derive(Clone)]
pub struct TessellateSource {
    pub url: String,
    pub filetype: String,
    pub max_zoom: ZoomLevel
}

impl TessellateSource {
    pub fn new(url: &str, filetype: &str, max_zoom: ZoomLevel) -> Self {
        Self {
            url: url.to_string(),
            filetype: filetype.to_string(),
            max_zoom,
        }
    }

    pub fn format(&self, coords: &WorldTileCoords) -> String {
        let tile_coords = coords.into_tile(TileAddressingScheme::XYZ).unwrap();
        format!(
            "{url}/{z}/{x}/{y}.{filetype}",
            url = self.url,
            z = tile_coords.z,
            x = tile_coords.x,
            y = tile_coords.y,
            filetype = self.filetype,
        )
    }
}

impl Default for TessellateSource {
    fn default() -> Self {
        Self::new("https://api.maptiler.com/tiles/v3-openmaptiles", "pbf?key=rzq14TmV1096yjvD3Uyw", ZoomLevel::new(14))
    }
}

/// Represents a source from which the raster tile are fetched.
#[derive(Clone)]
pub struct RasterSource {
    pub url: String,
    pub filetype: String,
    pub key: String,
}

impl RasterSource {
    pub fn new(url: &str, filetype: &str, key: &str) -> Self {
        Self {
            url: url.to_string(),
            filetype: filetype.to_string(),
            key: key.to_string(),
        }
    }

    pub fn format(&self, coords: &WorldTileCoords) -> String {
        let tile_coords = coords.into_tile(TileAddressingScheme::XYZ).unwrap();
        format!(
            "{url}/{z}/{x}/{y}.{filetype}?key={key}",
            url = self.url,
            z = tile_coords.z,
            x = tile_coords.x,
            y = tile_coords.y,
            filetype = self.filetype,
            key = self.key,
        )
    }
}

impl Default for RasterSource {
    fn default() -> Self {
        Self::new(
            "https://api.maptiler.com/tiles/satellite-v2",
            "jpg",
            "qnePkfbGpMsLCi3KFBs3",
        )
    }
}

/// Represents the tiles' different types of source.
#[derive(Clone)]
pub enum SourceType {
    Raster(RasterSource),
    Tessellate(TessellateSource),
}

impl SourceType {
    pub fn format(&self, coords: &WorldTileCoords) -> String {
        match self {
            SourceType::Raster(raster_source) => raster_source.format(coords),
            SourceType::Tessellate(tessellate_source) => tessellate_source.format(coords),
        }
    }
}
