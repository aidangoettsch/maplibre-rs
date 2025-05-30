//! Tessellator implementation.

use std::cell::RefCell;
use std::collections::HashMap;
use geozero::{ColumnValue, FeatureProcessor, GeomProcessor, PropertyProcessor};
use lyon::{
    geom,
    lyon_tessellation::VertexBuffers,
    path::{path::Builder, Path},
    tessellation::{
        geometry_builder::MaxIndex, BuffersBuilder, FillOptions, FillRule, FillTessellator,
        StrokeOptions, StrokeTessellator,
    },
};

use crate::{
    render::ShaderVertex,
    tessellation::{VertexConstructor, DEFAULT_TOLERANCE},
};
use crate::style::expression::{ComparisonLiteral, LegacyFilterExpression};

type GeoResult<T> = geozero::error::Result<T>;

/// Build tessellations with vectors.
pub struct ZeroTessellator<I: std::ops::Add + From<lyon::tessellation::VertexId> + MaxIndex> {
    path_builder: RefCell<Builder>,
    path_open: bool,
    is_point: bool,

    pub buffer: VertexBuffers<ShaderVertex, I>,

    pub feature_indices: Vec<u32>,
    current_index: usize,
    
    filter: Option<LegacyFilterExpression>,
    properties: HashMap<String, ComparisonLiteral>,
    filtered: bool,
}

impl<I: std::ops::Add + From<lyon::tessellation::VertexId> + MaxIndex> Default
    for ZeroTessellator<I>
{
    fn default() -> Self {
        Self {
            path_builder: RefCell::new(Path::builder()),
            buffer: VertexBuffers::new(),
            feature_indices: Vec::new(),
            current_index: 0,
            path_open: false,
            is_point: false,
            filter: None,
            properties: Default::default(),
            filtered: false,
        }
    }
}

impl<I: std::ops::Add + From<lyon::tessellation::VertexId> + MaxIndex> ZeroTessellator<I> {
    pub fn new(filter: Option<LegacyFilterExpression>) -> Self {
        Self {
            path_builder: RefCell::new(Path::builder()),
            buffer: VertexBuffers::new(),
            feature_indices: Vec::new(),
            current_index: 0,
            path_open: false,
            is_point: false,
            filter,
            properties: Default::default(),
            filtered: false,
        }
    }
    
    fn cur_feature_matches_filter(&self) -> bool {
        self.filter.as_ref().is_none_or(|filter| filter.evaluate(&self.properties))
    }
    
    fn update_feature_indices(&mut self) {
        let next_index = self.buffer.indices.len();
        let indices = (next_index - self.current_index) as u32;
        self.feature_indices.push(indices);
        self.current_index = next_index;
    }

    fn tessellate_strokes(&mut self) {
        let path_builder = self.path_builder.replace(Path::builder());

        self.properties.insert("$type".to_string(), ComparisonLiteral::String("LineString".to_string()));
        if !self.cur_feature_matches_filter() {
            self.filtered = true;
            return
        }
        
        log::info!("UNFILTERED LINE FILTER WAS {:?}\nTHIS LINE HAS PROPS {:?}", self.filter, self.properties);

        StrokeTessellator::new()
            .tessellate_path(
                &path_builder.build(),
                &StrokeOptions::tolerance(DEFAULT_TOLERANCE),
                &mut BuffersBuilder::new(&mut self.buffer, VertexConstructor {}),
            )
            .unwrap(); // TODO: Remove unwrap
    }

    fn end(&mut self, close: bool) {
        if self.path_open {
            self.path_builder.borrow_mut().end(close);
            self.path_open = false;
        }
    }

    fn tessellate_fill(&mut self) {
        let path_builder = self.path_builder.replace(Path::builder());
        
        self.properties.insert("$type".to_string(), ComparisonLiteral::String("Polygon".to_string()));
        if !self.cur_feature_matches_filter() {
            self.filtered = true;
            return
        }
        log::info!("UNFILTERED FILL FILTER WAS {:?}\nTHIS FILL HAS PROPS {:?}", self.filter, self.properties);

        FillTessellator::new()
            .tessellate_path(
                &path_builder.build(),
                &FillOptions::tolerance(DEFAULT_TOLERANCE).with_fill_rule(FillRule::NonZero),
                &mut BuffersBuilder::new(&mut self.buffer, VertexConstructor {}),
            )
            .unwrap(); // TODO: Remove unwrap
    }
}

impl<I: std::ops::Add + From<lyon::tessellation::VertexId> + MaxIndex> GeomProcessor
    for ZeroTessellator<I>
{
    fn xy(&mut self, x: f64, y: f64, _idx: usize) -> GeoResult<()> {
        // log::info!("xy");

        if self.is_point {
            // log::info!("point");
        } else if !self.path_open {
            self.path_builder
                .borrow_mut()
                .begin(geom::point(x as f32, y as f32));
            self.path_open = true;
        } else {
            self.path_builder
                .borrow_mut()
                .line_to(geom::point(x as f32, y as f32));
        }
        Ok(())
    }

    fn point_begin(&mut self, _idx: usize) -> GeoResult<()> {
        // log::info!("point_begin");
        self.is_point = true;
        Ok(())
    }

    fn point_end(&mut self, _idx: usize) -> GeoResult<()> {
        // log::info!("point_end");
        self.is_point = false;
        Ok(())
    }

    fn multipoint_begin(&mut self, _size: usize, _idx: usize) -> GeoResult<()> {
        // log::info!("multipoint_begin");
        Ok(())
    }

    fn multipoint_end(&mut self, _idx: usize) -> GeoResult<()> {
        // log::info!("multipoint_end");
        Ok(())
    }

    fn linestring_begin(&mut self, _tagged: bool, _size: usize, _idx: usize) -> GeoResult<()> {
        // log::info!("linestring_begin");
        Ok(())
    }

    fn linestring_end(&mut self, tagged: bool, _idx: usize) -> GeoResult<()> {
        // log::info!("linestring_end");

        self.end(false);

        if tagged {
            self.tessellate_strokes();
        }
        Ok(())
    }

    fn multilinestring_begin(&mut self, _size: usize, _idx: usize) -> GeoResult<()> {
        // log::info!("multilinestring_begin");
        Ok(())
    }

    fn multilinestring_end(&mut self, _idx: usize) -> GeoResult<()> {
        // log::info!("multilinestring_end");
        self.tessellate_strokes();
        Ok(())
    }

    fn polygon_begin(&mut self, _tagged: bool, _size: usize, _idx: usize) -> GeoResult<()> {
        // log::info!("polygon_begin");
        Ok(())
    }

    fn polygon_end(&mut self, tagged: bool, _idx: usize) -> GeoResult<()> {
        // log::info!("polygon_end");

        self.end(true);
        if tagged {
            self.tessellate_fill();
        }
        Ok(())
    }

    fn multipolygon_begin(&mut self, _size: usize, _idx: usize) -> GeoResult<()> {
        // log::info!("multipolygon_begin");
        Ok(())
    }

    fn multipolygon_end(&mut self, _idx: usize) -> GeoResult<()> {
        // log::info!("multipolygon_end");

        self.tessellate_fill();
        Ok(())
    }
}

impl<I: std::ops::Add + From<lyon::tessellation::VertexId> + MaxIndex> PropertyProcessor
    for ZeroTessellator<I>
{
    fn property(&mut self, _idx: usize, name: &str, value: &ColumnValue) -> geozero::error::Result<bool> {
        self.properties.insert(name.to_string(), value.into());
        Ok(true)
    }
}

impl<I: std::ops::Add + From<lyon::tessellation::VertexId> + MaxIndex> FeatureProcessor
    for ZeroTessellator<I>
{
    fn feature_begin(&mut self, _idx: u64) -> geozero::error::Result<()> {
        self.properties.clear();
        self.filtered = false;
        Ok(())
    }
    
    fn feature_end(&mut self, _idx: u64) -> geozero::error::Result<()> {
        if !self.filtered {
            self.update_feature_indices();
        }
        Ok(())
    }
}
