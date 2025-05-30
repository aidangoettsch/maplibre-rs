//! Specifies the instructions which are going to be sent to the GPU. Render commands can be concatenated
//! into a new render command which executes multiple instruction sets.
use crate::{
    render::{
        eventually::{Eventually, Eventually::Initialized},
        render_phase::{LayerItem, PhaseItem, RenderCommand, RenderCommandResult},
        resource::TrackedRenderPass,
        tile_view_pattern::WgpuTileViewPattern,
        INDEX_FORMAT,
    },
    tcs::world::World,
    vector::{VectorBufferPool, VectorPipeline},
};

pub struct SetVectorTilePipeline;
impl<P: PhaseItem> RenderCommand<P> for SetVectorTilePipeline {
    fn render<'w>(
        world: &'w World,
        _item: &P,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(Initialized(pipeline)) = world.resources.get::<Eventually<VectorPipeline>>()
        else {
            return RenderCommandResult::Failure;
        };

        pass.set_render_pipeline(pipeline);
        RenderCommandResult::Success
    }
}

pub struct DrawVectorTile;
impl RenderCommand<LayerItem> for DrawVectorTile {
    fn render<'w>(
        world: &'w World,
        item: &LayerItem,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some((Initialized(buffer_pool), Initialized(tile_view_pattern))) =
            world.resources.query::<(
                &Eventually<VectorBufferPool>,
                &Eventually<WgpuTileViewPattern>,
            )>()
        else {
            return RenderCommandResult::Failure;
        };

        let Some(vector_layers) = buffer_pool.index().get_layers(item.tile.coords) else {
            return RenderCommandResult::Failure;
        };

        let Some(entry) = vector_layers
            .iter()
            .find(|entry| entry.style_layer.id == item.style_layer)
        else {
            log::error!("Rendering {} failed because the original entry couldn't be found", item.style_layer);
            return RenderCommandResult::Failure;
        };

        let source_shape = &item.source_shape;

        // Uses stencil value of requested tile and the shape of the requested tile
        let reference = source_shape.coords().stencil_reference_value_3d() as u32;

        let index_range = entry.indices_buffer_range();
        let vertex_range = entry.vertices_buffer_range();
        let layer_meta_range = entry.layer_metadata_buffer_range();
        let feature_meta_range = entry.feature_metadata_buffer_range();
        
        log::info!(
            "Drawing layer {:?} at {} with index len {} vertex len {} layer meta len {} feature meta len {}",
            entry.style_layer.id,
            entry.coords,
            index_range.end - index_range.start,
            vertex_range.end - vertex_range.start,
            layer_meta_range.end - layer_meta_range.start,
            feature_meta_range.end - feature_meta_range.start,
        );

        if index_range.is_empty() {
            log::error!("Tried to draw a vector tile without any vertices");
            return RenderCommandResult::Failure;
        }

        pass.set_stencil_reference(reference);

        pass.set_index_buffer(buffer_pool.indices().slice(index_range), INDEX_FORMAT);
        pass.set_vertex_buffer(
            0,
            buffer_pool.vertices().slice(entry.vertices_buffer_range()),
        );
        let tile_view_pattern_buffer = source_shape
            .buffer_range()
            .expect("tile_view_pattern needs to be uploaded first"); // FIXME tcs
        pass.set_vertex_buffer(
            1,
            tile_view_pattern.buffer().slice(tile_view_pattern_buffer),
        );
        pass.set_vertex_buffer(
            2,
            buffer_pool
                .metadata()
                .slice(entry.layer_metadata_buffer_range()),
        );
        pass.set_vertex_buffer(
            3,
            buffer_pool
                .feature_metadata()
                .slice(entry.feature_metadata_buffer_range()),
        );
        pass.draw_indexed(entry.indices_range(), 0, 0..1);

        log::info!("Drawing layer {} DONE", entry.style_layer.id);

        RenderCommandResult::Success
    }
}

pub type DrawVectorTiles = (SetVectorTilePipeline, DrawVectorTile);
