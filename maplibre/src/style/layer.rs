//! Vector tile layer drawing utilities.

use std::collections::HashMap;
use cgmath::num_traits::pow;
use cint::{Alpha, EncodedSrgb};
use csscolorparser::Color;
use serde::{Deserialize, Serialize};
use crate::coords::ZoomLevel;
use crate::style::raster::RasterLayer;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum InterpolatedQuantity<T> {
    Fixed(T),
    Interpolated {
        base: T,
        stops: Vec<(ZoomLevel, T)>
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
            match interpolant {
                InterpolatedQuantity::Fixed(alpha) => {
                    c.alpha = *alpha;
                }
                InterpolatedQuantity::Interpolated { base, stops } => { 
                    if stops.is_empty() {
                        return c
                    }
                    
                    let (min_zoom, min_zoom_value) = stops.first().unwrap();
                    let (max_zoom, max_zoom_value) = stops.last().unwrap();
                    
                    let window = stops
                        .iter()
                        .zip(stops.iter().skip(1))
                        .find(|((stop_a, _), (stop_b, _))| *stop_a <= zoom_level && *stop_b >= zoom_level);
                    
                    let alpha = if let Some(((stop_a, stop_a_value), (stop_b, stop_b_value))) = window {
                        let zoom_diff: ZoomLevel = *stop_b - (*stop_a).into();
                        let zoom_prog: ZoomLevel = zoom_level - (*stop_a).into();

                        let zoom_diff_u8: u8 = zoom_diff.into();
                        let zoom_prog_u8: u8 = zoom_prog.into();
                        
                        let interp_factor = if zoom_diff == ZoomLevel::new(0) {
                            0f32
                        } else if *base == 1.0 {
                            (zoom_diff_u8 as f32) / (zoom_prog_u8 as f32)
                        } else {
                            (pow(*base, zoom_prog_u8.into()) - 1.0) / (pow(*base, zoom_diff_u8.into()) - 1.0)
                        };
                        
                        stop_a_value + (stop_b_value - stop_a_value) * interp_factor
                    } else if zoom_level <= *min_zoom {
                        *min_zoom_value
                    } else {
                        *max_zoom_value
                    };
                    
                    c.alpha = alpha;
                },
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
}

impl Default for StyleLayer {
    fn default() -> Self {
        Self {
            index: 0,
            id: "id".to_string(),
            maxzoom: None,
            minzoom: None,
            metadata: None,
            paint: None,
            source: None,
            source_layer: Some("does not exist".to_string()),
        }
    }
}
