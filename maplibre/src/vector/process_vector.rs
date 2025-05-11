use std::{borrow::Cow, collections::HashSet, marker::PhantomData};

use geozero::{
    mvt::{tile, Message},
    GeozeroDatasource,
};
use thiserror::Error;

use crate::{
    coords::WorldTileCoords,
    io::{
        apc::{Context, SendError},
        // geometry_index::{IndexProcessor, IndexedGeometry, TileIndex},
        geometry_index::{IndexedGeometry, TileIndex},
    },
    render::ShaderVertex,
    tessellation::{zero_tessellator::ZeroTessellator, IndexDataType, OverAlignedVertexBuffer},
    vector::transferables::{
        LayerIndexed, LayerMissing, LayerTessellated, TileTessellated, VectorTransferables,
    },
};
use crate::style::layer::StyleLayer;
use crate::style::Style;

#[derive(Error, Debug)]
pub enum ProcessVectorError {
    /// Sending of results failed
    #[error("sending data back through context failed")]
    SendError(SendError),
    /// Error when decoding e.g. the protobuf file
    #[error("decoding failed")]
    Decoding(Cow<'static, str>),
}

/// A request for a tile at the given coordinates and in the given layers.
pub struct VectorTileRequest {
    pub coords: WorldTileCoords,
    pub layers: HashSet<String>,
    pub style: Style,
}

pub fn process_vector_tile<T: VectorTransferables, C: Context>(
    data: &[u8],
    tile_request: VectorTileRequest,
    context: &mut ProcessVectorContext<T, C>,
) -> Result<(), ProcessVectorError> {
    // Decode

    let mut tile = geozero::mvt::Tile::decode(data)
        .map_err(|e| ProcessVectorError::Decoding(e.to_string().into()))?;

    // Available

    let coords = &tile_request.coords;

    for layer in &mut tile.layers {
        let layer_name: &str = &layer.name;
        if !tile_request.layers.contains(layer_name) {
            continue;
        }
        
        let corresponding_style_layers: Vec<&StyleLayer> = tile_request.style.layers
            .iter()
            .filter(|style_layer| style_layer.source_layer
                .as_ref()
                .is_some_and(|source| source.as_str() == layer_name)
            )
            .collect();
        
        for style_layer in corresponding_style_layers {
            let mut layer = layer.clone();
            log::info!("Processing layer {} with filter {:?}", style_layer.id, &style_layer.filter);
            let mut tessellator = ZeroTessellator::<IndexDataType>::new(style_layer.filter.clone());
            if let Err(e) = layer.process(&mut tessellator) {
                context.layer_missing(coords, style_layer.id.as_str())?;

                log::error!("layer {} at {coords} tesselation failed {e:?}", style_layer.id.as_str());
            } else {
                if let Err(e) = context.layer_tesselation_finished(
                    coords,
                    tessellator.buffer.into(),
                    tessellator.feature_indices,
                    layer,
                    style_layer.id.clone()
                ) {
                    context.layer_missing(coords, style_layer.id.as_str())?;

                    log::error!("layer {} at {coords} failed to send tesselation finished {e:?}", style_layer.id.as_str());
                }
            }
        }
    }

    // Missing

    let coords = &tile_request.coords;
    
    let available_layers: HashSet<_> = tile
        .layers
        .iter()
        .map(|layer| layer.name.clone())
        .collect::<HashSet<_>>();
    
    for missing_layer in tile_request.layers.difference(&available_layers) {
        context.layer_missing(coords, missing_layer)?;
        log::error!("requested layer {missing_layer} at {coords} not found in tile");
    }

    // Indexing

    // let mut index = IndexProcessor::new();
    // 
    // for layer in &mut tile.layers {
    //     layer.process(&mut index).unwrap();
    // }
    // 
    // context.layer_indexing_finished(&tile_request.coords, index.get_geometries())?;

    // End

    tracing::info!("tile tessellated at {coords} finished");
    context.tile_finished(coords)?;

    Ok(())
}

pub struct ProcessVectorContext<T: VectorTransferables, C: Context> {
    context: C,
    phantom_t: PhantomData<T>,
}

impl<T: VectorTransferables, C: Context> ProcessVectorContext<T, C> {
    pub fn new(context: C) -> Self {
        Self {
            context,
            phantom_t: Default::default(),
        }
    }
}

impl<T: VectorTransferables, C: Context> ProcessVectorContext<T, C> {
    pub fn take_context(self) -> C {
        self.context
    }

    fn tile_finished(&mut self, coords: &WorldTileCoords) -> Result<(), ProcessVectorError> {
        self.context
            .send_back(T::TileTessellated::build_from(*coords))
            .map_err(|e| ProcessVectorError::SendError(e))
    }

    fn layer_missing(
        &mut self,
        coords: &WorldTileCoords,
        layer_name: &str,
    ) -> Result<(), ProcessVectorError> {
        self.context
            .send_back(T::LayerMissing::build_from(*coords, layer_name.to_owned()))
            .map_err(|e| ProcessVectorError::SendError(e))
    }

    fn layer_tesselation_finished(
        &mut self,
        coords: &WorldTileCoords,
        buffer: OverAlignedVertexBuffer<ShaderVertex, IndexDataType>,
        feature_indices: Vec<u32>,
        layer_data: tile::Layer,
        style_layer_id: String
    ) -> Result<(), ProcessVectorError> {
        self.context
            .send_back(T::LayerTessellated::build_from(
                *coords,
                buffer,
                feature_indices,
                layer_data,
                style_layer_id,
            ))
            .map_err(|e| ProcessVectorError::SendError(e))
    }

    fn layer_indexing_finished(
        &mut self,
        coords: &WorldTileCoords,
        geometries: Vec<IndexedGeometry<f64>>,
    ) -> Result<(), ProcessVectorError> {
        self.context
            .send_back(T::LayerIndexed::build_from(
                *coords,
                TileIndex::Linear { list: geometries },
            ))
            .map_err(|e| ProcessVectorError::SendError(e))
    }
}

#[cfg(test)]
mod tests {
    use super::ProcessVectorContext;
    use crate::{
        coords::ZoomLevel,
        io::apc::tests::DummyContext,
        vector::{
            process_vector::{process_vector_tile, VectorTileRequest},
            DefaultVectorTransferables,
        },
    };

    #[test] // TODO: Add proper tile byte array
    #[ignore]
    fn test() {
        let _output = process_vector_tile(
            &[0],
            VectorTileRequest {
                coords: (0, 0, ZoomLevel::default()).into(),
                layers: Default::default(),
                style: Default::default()
            },
            &mut ProcessVectorContext::<DefaultVectorTransferables, _>::new(DummyContext),
        );
    }
}
