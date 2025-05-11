//! Vector tile layer drawing utilities.

use std::collections::HashMap;
use cint::{Alpha, EncodedSrgb};
use csscolorparser::Color;
use serde::{Deserialize, Serialize};
use crate::coords::ZoomLevel;
use crate::style::expression::LegacyFilterExpression;
use crate::style::raster::RasterLayer;
use crate::style::util::interpolate;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum InterpolatedQuantity<T> {
    Fixed(T),
    Interpolated {
        base: T,
        stops: Vec<(f64, T)>
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackgroundPaint {
    #[serde(rename = "background-color")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<Color>,
    #[serde(rename = "background-opacity")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_opacity: Option<InterpolatedQuantity<f32>>,
    // TODO a lot
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FillPaint {
    #[serde(rename = "fill-color")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Color>,
    #[serde(rename = "fill-opacity")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_opacity: Option<InterpolatedQuantity<f32>>,
    // TODO a lot
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LinePaint {
    #[serde(rename = "line-color")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_color: Option<Color>,
    #[serde(rename = "line-opacity")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_opacity: Option<InterpolatedQuantity<f32>>,
    #[serde(rename = "line-width")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_width: Option<InterpolatedQuantity<f32>>,
    // TODO a lot
}

/// The different types of paints.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "paint")]
pub enum LayerPaint {
    #[serde(rename = "background")]
    Background(BackgroundPaint),
    #[serde(rename = "line")]
    Line(LinePaint),
    #[serde(rename = "fill")]
    Fill(FillPaint),
    #[serde(rename = "raster")]
    Raster(RasterLayer),
}

fn cint_color_from_css_color_and_opacity(css_color: &Option<Color>, opacity: &Option<InterpolatedQuantity<f32>>, zoom_level: ZoomLevel) -> Option<Alpha<EncodedSrgb<f32>>> {
    let color: Option<Alpha<EncodedSrgb<f32>>> = css_color
        .as_ref()
        .map(|color| color.clone().into());

    color.map(|mut c| {
        if let Some(interpolant) = opacity {
            if let Some(alpha) = interpolate(interpolant, zoom_level) {
                c.alpha = alpha;
            }
        }
        
        c
    })
}

impl LayerPaint {
    pub fn get_color(&self, zoom_level: ZoomLevel) -> Option<Alpha<EncodedSrgb<f32>>> {
        match self {
            LayerPaint::Background(paint) => cint_color_from_css_color_and_opacity(&paint.background_color, &paint.background_opacity, zoom_level),
            LayerPaint::Line(paint) => cint_color_from_css_color_and_opacity(&paint.line_color, &paint.line_opacity, zoom_level),
            LayerPaint::Fill(paint) => cint_color_from_css_color_and_opacity(&paint.fill_color, &paint.fill_opacity, zoom_level),
            LayerPaint::Raster(_) => None,
        }
    }
}

/// Stores all the styles for a specific layer.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StyleLayer {
    #[serde(skip)]
    pub index: u32,
    pub id: String,
    // TODO filter
    // TODO layout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maxzoom: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minzoom: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
    pub paint: Option<LayerPaint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename="source-layer")]
    pub source_layer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<LegacyFilterExpression>,
}

impl Default for StyleLayer {
    fn default() -> Self {
        Self {
            index: 0,
            id: "id".to_string(),
            maxzoom: None,
            minzoom: None,
            filter: None,
            metadata: None,
            paint: None,
            source: None,
            source_layer: Some("does not exist".to_string()),
        }
    }
}
