use crate::coords::ZoomLevel;
use crate::style::layer::InterpolatedQuantity;

pub fn interpolate(quantity: &InterpolatedQuantity<f32>, zoom_level: ZoomLevel) -> Option<f32> {
    let zoom_level = <ZoomLevel as Into<f64>>::into(zoom_level);
    
    match quantity {
        InterpolatedQuantity::Fixed(val) => Some(*val),
        InterpolatedQuantity::Interpolated { base, stops } => {
            if stops.is_empty() {
                log::info!("empty stops! {:?}", stops);
                return None
            }

            let (min_zoom, min_zoom_value) = stops.first().unwrap();
            let (max_zoom, max_zoom_value) = stops.last().unwrap();

            let window = stops
                .iter()
                .zip(stops.iter().skip(1))
                .find(|((stop_a, _), (stop_b, _))| *stop_a <= zoom_level && *stop_b >= zoom_level);

            if let Some(((stop_a, stop_a_value), (stop_b, stop_b_value))) = window {
                let zoom_diff: f64 = *stop_b - *stop_a;
                let zoom_prog: f64 = zoom_level - *stop_a;

                let interp_factor = if zoom_diff == 0.0 {
                    0.0f32
                } else if *base == 1.0 {
                    (zoom_prog as f32) / (zoom_diff as f32)
                } else {
                    (base.powf(zoom_prog as f32) - 1.0) / (base.powf(zoom_diff as f32) - 1.0)
                };

                Some(*stop_a_value + (*stop_b_value - *stop_a_value) * interp_factor)
            } else if zoom_level <= *min_zoom {
                Some(*min_zoom_value)
            } else {
                Some(*max_zoom_value)
            }
        }
    }
}