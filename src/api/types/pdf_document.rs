//! A `PDFDocument` represents the whole content of the file

extern crate lopdf;

use super::*;
use super::super::traits::*;

use errors::*;
use api::types::plugins::graphics::two_dimensional::*;
use api::types::plugins::graphics::*;
use std::io::{Write, Seek};

/// PDF document
#[derive(Debug)]
pub struct PdfDocument {
    // Pages of the document
    pages: Vec<PdfPage>,
    /// PDF document title
    title: String,
    /// PDF creator name
    creator: String,
    /// PDF contents (subject to change)
    contents: Vec<Box<IntoPdfObject>>,
    /// Inner PDF document
    inner_doc: lopdf::Document,
    /// Current PDF marker (where we are in the document)
    current_marker: PdfMarkerIndex,
}

impl<'a> PdfDocument {

    /// Creates a new PDF document
    #[inline]
    pub fn new<S>(initial_page: PdfPage, title: S, creator: S)
    -> Self where S: Into<String>
    {
        let title_str = title.into();
        let creator_str = creator.into();
        let mut inner_doc = lopdf::Document::new();

        Self {
            pages: vec![initial_page],
            title: title_str,
            creator: creator_str,
            contents: Vec::new(),
            inner_doc: inner_doc,
            current_marker: (0, 0, 0),
        }
    }

    /// # `add_*` functions

    /// Create a new pdf page and returns the index of the page
    #[inline]
    pub fn add_page(&mut self, x_mm: f64, y_mm: f64)
    -> PdfPageIndex
    {
        self.pages.push(PdfPage::new(x_mm, y_mm));
        self.pages.len() - 1
    }

    /// Create a new pdf layer and returns the index to it
    #[inline]
    pub fn add_layer<S>(&mut self, name: S, page: &PdfPageIndex)
    -> ::std::result::Result<PdfLayerIndex, Error> where S: Into<String>
    {
        let layer_index = self.get_mut_page(page)?
                              .add_layer(name);
        Ok((*page, layer_index))
    }

    /// Create a new marker on the layer. Error if the page does not exist
    #[inline]
    pub fn add_marker(&mut self, x_mm: f64, y_mm: f64, layer: &PdfLayerIndex)
    -> ::std::result::Result<PdfMarkerIndex, Error>
    {
        let marker_index = self.get_mut_page(&layer.0)?
                               .get_mut_layer(&layer.1)?
                               .add_marker(x_mm, y_mm);
        Ok((layer.0, layer.1, marker_index))
    }

    /// Add arbitrary Pdf Objects. These are tracked by reference and get 
    /// instantiated / referenced when the document is saved.
    #[inline]
    pub fn add_arbitrary_content<C>(&mut self, content: Box<C>)
    -> PdfContentIndex where C: 'static + IntoPdfObject
    {
        self.contents.push(content);
        self.contents.len() - 1
    }

    /// ## `add_*` functions for arbitrary PDF content

    /// Add a font from a 
    #[inline]
    pub fn add_font<R>(&mut self, font_stream: R)
    -> ::std::result::Result<FontIndex, Error> where R: ::std::io::Read
    {
        use api::types::plugins::graphics::two_dimensional::Font;
        let font = Font::new(font_stream)?;
        let index = self.add_arbitrary_content(Box::new(font));
        Ok(FontIndex(index))
    }

    /// Add text to the file
    #[inline]
    pub fn add_text<S>(&mut self, 
                      text: S, 
                      font: FontIndex, 
                      font_size: usize, 
                      position: &PdfMarkerIndex)
    -> ::std::result::Result<(), Error> where S: Into<String>
    {
        // todo
        Ok(())
    }

    /// Add a line to the document
    #[inline]
    pub fn add_line(&mut self,
                    points: Vec<(Point, bool)>, 
                    layer: &PdfLayerIndex, 
                    outline: Option<&Outline>, 
                    fill: Option<&Fill>)
    -> ::std::result::Result<(), Error>
    {
        // todo
        Ok(())
    }

    /// Add SVG content to the document
    #[inline]
    pub fn add_svg<R>(&mut self,
                      svg_data: R)
    -> ::std::result::Result<SvgIndex, Error> 
    where R: ::std::io::Read
    {
        // todo
        unimplemented!()
    }

    /// Instantiate SVG data
    #[inline]
    pub fn add_svg_at(&mut self,
                      svg_data_index: &SvgIndex,
                      width_mm: f64,
                      height_mm: f64,
                      position: &PdfMarkerIndex)
    {
        // todo
    }

    /// # `get_*` functions

    /// Validates that a page is accessible and returns the page
    #[inline]
    pub fn get_page(&self, page: &PdfPageIndex)
    -> ::std::result::Result<&PdfPage, Error>
    {
        use errors::index_error::ErrorKind::*;
        self.pages.get(*page)
                  .ok_or(Error::from_kind(IndexError(PdfPageIndexError)))
    }

    /// Validates that a page is accessible and returns the page
    #[inline]
    pub fn get_mut_page(&mut self, page: &PdfPageIndex)
    -> ::std::result::Result<&mut PdfPage, Error>
    {
        use errors::index_error::ErrorKind::*;
        self.pages.get_mut(*page)
                  .ok_or(Error::from_kind(IndexError(PdfPageIndexError)))
    }

    /// Validates that a layer is accessible and returns the layer
    #[inline]
    pub fn get_layer(&self, layer: &PdfLayerIndex)
    -> ::std::result::Result<&PdfLayer, Error>
    {
        let layer = self.get_page(&layer.0)?
                        .get_layer(&layer.1)?;
        Ok(layer)
    }

    /// Validates that a layer is accessible and returns the mutable layer
    #[inline]
    pub fn get_mut_layer(&mut self, layer: &PdfLayerIndex)
    -> ::std::result::Result<&mut PdfLayer, Error>
    {
        let layer = self.get_mut_page(&layer.0)?
                        .get_mut_layer(&layer.1)?;
        Ok(layer)
    }

    /// Validates that a marker is present and returns the marker
    #[inline]
    pub fn get_marker(&self, marker: &PdfMarkerIndex)
    -> ::std::result::Result<&PdfMarker, Error>
    {
        let marker = self.get_page(&marker.0)?
                         .get_layer(&marker.1)?
                         .get_marker(&marker.2)?;
         Ok(marker)
    }

    /// Validates that a marker is present and returns the marker
    #[inline]
    pub fn get_mut_marker(&mut self, marker: &PdfMarkerIndex)
    -> ::std::result::Result<&mut PdfMarker, Error>
    {
        let marker = self.get_mut_page(&marker.0)?
                         .get_mut_layer(&marker.1)?
                         .get_mut_marker(&marker.2)?;
         Ok(marker)
    }

    /// Drops the PDFDocument, returning the inner `lopdf::Document`. 
    /// Document may be only half-written
    #[inline]
    pub fn get_inner(self)
    -> (lopdf::Document, Vec<Box<IntoPdfObject>>, PdfMarker)
    {

        let marker = self.get_marker(&self.current_marker).unwrap().clone();
        (self.inner_doc, self.contents, marker)
    }

    /// ## Miscellaneous functions

    /// Sets the current PDF marker
    #[inline]
    pub fn set_current_marker(&mut self, marker: &PdfMarkerIndex)
    {
        self.current_marker = *marker;
    }

    /// Save PDF Document, writing the contents to the target
    pub fn save<W: Write + Seek>(mut self, target: &mut W)
    -> ::std::result::Result<(), Error>
    {
        use lopdf::{Dictionary, Object};
        use lopdf::Object::{Integer, Reference};
        use std::iter::FromIterator;

        // add root, catalog & pages
        let start = self.inner_doc.new_object_id();

        let catalog = Dictionary::from_iter(vec![
                      ("Type", "Catalog".into()),
                      ("PageLayout", "OneColumn".into()),
                      ("PageMode", "Use0".into()),
                      ("Pages", Reference(start)), ]);

        let mut pages = Dictionary::from_iter(vec![
                      ("Type", "Pages".into()),
                      ("Count", Integer(self.pages.len() as i64)),
                      /* Resources, Fonts */
                      ]);
        
        // add all contents, save references
        // todo

        // add pages
        let mut page_ids = Vec::<Object>::new();

        for page in self.pages.clone().into_iter() {
            
            let mut p = Dictionary::from_iter(vec![
                      ("Type", "Page".into()),
                      ("MediaBox", vec![0.into(), 0.into(),
                       page.width_pt.into(), page.heigth_pt.into()].into()),
                      ("Parent", Reference(start)),
                      /* todo: ArtBox */ ]);

            p.set("Parent", Reference(start));

            // add page content references
            // todo

            page_ids.push(Reference(self.inner_doc.add_object(p)))
        }

        pages.set::<String, Object>("Kids".into(), page_ids.into());

        // save inner document
        let catalog_id = self.inner_doc.add_object(catalog);
        self.inner_doc.trailer.set("Root", Reference(catalog_id));
        self.inner_doc.compress();
        self.inner_doc.save_to(target).unwrap();

        Ok(())
    }
}

/*
impl std::convert::From<lopdf::Doument> for PdfDocument
{
    fn from(doc: lopdf::Doument) -> Self
    {
        
    }
}
*/