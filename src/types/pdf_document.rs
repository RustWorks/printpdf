//! A `PDFDocument` represents the whole content of the file

extern crate lopdf;
extern crate chrono;
extern crate rand;

use *;
use types::indices::*;
use std::io::{Write, Seek};
use rand::Rng;
use std::sync::{Arc, Mutex};

/// PDF document
#[derive(Debug)]
pub struct PdfDocument {
    /// Pages of the document
    pages: Vec<PdfPage>,
    /// PDF contents as references.
    /// As soon as data gets added to the inner_doc, a reference gets pushed into here
    #[doc_hidden]
    pub(super) contents: Vec<lopdf::Object>,
    /// Inner PDF document
    #[doc_hidden]
    pub(super) inner_doc: lopdf::Document,
    /// Document ID. Must be changed if the document is loaded / parsed from a file
    pub document_id: std::string::String,
    /// Metadata for this document
    pub metadata: PdfMetadata,
}

impl PdfDocument {

    /// Creates a new PDF document
    #[inline]
    pub fn new<S>(document_title: S,
                  initial_page_width_mm: f64, 
                  initial_page_height_mm: f64, 
                  initial_layer_name: S)
    -> (Arc<Mutex<Self>>, PdfPageIndex, PdfLayerIndex) where S: Into<String>
    {
        let mut doc = Self {
            pages: Vec::new(),
            document_id: rand::thread_rng().gen_ascii_chars().take(32).collect(),
            contents: Vec::new(),
            inner_doc: lopdf::Document::with_version("1.3"),
            metadata: PdfMetadata::new(document_title, 1, false, PdfConformance::X3_2003_PDF_1_4)
        };

        let doc_ref = Arc::new(Mutex::new(doc));

        let (initial_page, layer_index) = PdfPage::new(
            Arc::downgrade(&doc_ref), 
            initial_page_width_mm, 
            initial_page_height_mm, 
            initial_layer_name);

        { doc_ref.lock().unwrap().pages.push(initial_page); }

        (doc_ref, PdfPageIndex(0), layer_index)
    }

    /// Checks for invalid settings in the document
    pub fn check_for_errors(&mut self) 
    -> ::std::result::Result<(), Error>
    {
        // todo
        Ok(())
    }

    /// Tries to match the document to the given conformance.
    /// Errors only on an unrecoverable error.
    pub fn repair_errors(&mut self, conformance: PdfConformance)
    -> ::std::result::Result<(), Error>
    {
        //todo
        Ok(())
    }

    // ----- BUILDER FUNCTIONS

    /// Set the trapping of the document
    #[inline]
    pub fn with_trapping(mut self, trapping: bool)
    -> Self 
    {
        self.metadata.trapping = trapping;
        self
    }

    /// Sets the document ID (for comparing two PDF documents for equality)
    #[inline]
    pub fn with_document_id(mut self, id: String)
    -> Self
    {
        self.metadata.xmp_metadata.document_id = id;
        self
    }

    /// Set the version of the document
    #[inline]
    pub fn with_document_version(mut self, version: u32)
    -> Self 
    {
        self.metadata.document_version = version;
        self
    }

    /// Changes the conformance of this document. It is recommended to call 
    /// `check_for_errors()` after changing it.
    #[inline]
    pub fn with_conformance(mut self, conformance: PdfConformance)
    -> Self
    {
        self.metadata.conformance = conformance;
        self
    }

    /// Sets the modification date on the document. Intended to be used when
    /// reading documents that already have a modification date.
    #[inline]
    pub fn with_mod_date(mut self, mod_date: chrono::DateTime<chrono::Local>)
    -> Self
    {
        self.metadata.modification_date = mod_date;
        self
    }

    // ----- ADD FUNCTIONS

    /// Create a new pdf page and returns the index of the page
    #[inline]
    pub fn add_page<S>(&mut self, x_mm: f64, y_mm: f64, inital_layer_name: S)
    -> (PdfPageIndex, PdfLayerIndex) where S: Into<String>
    {
        /* temporary, there has to be at least one root node */
        let document_weak_ptr = self.pages[0].document.clone();
        let (pdf_page, pdf_layer_index) = 
            PdfPage::new(document_weak_ptr, x_mm, y_mm, inital_layer_name);

        self.pages.push(pdf_page);
        (PdfPageIndex(self.pages.len() - 1), pdf_layer_index)
    }

    /// Add arbitrary Pdf Objects. These are tracked by reference and get 
    /// instantiated / referenced when the document is saved.
    #[inline]
    pub fn add_arbitrary_content<C>(&mut self, content: Box<C>)
    -> PdfContentIndex where C: 'static + IntoPdfObject
    {
        let obj_id = self.inner_doc.add_object(content.into_obj());
        self.contents.place_back() <- lopdf::Object::Reference(obj_id);
        PdfContentIndex(self.contents.len() - 1)
    }

    /// Add a font from a font stream
    #[inline]
    pub fn add_font<R>(&mut self, font_stream: R)
    -> ::std::result::Result<FontIndex, Error> where R: ::std::io::Read
    {
        let font = Font::new(font_stream)?;
        let index = self.add_arbitrary_content(Box::new(font));
        Ok(FontIndex(index))
    }

    /// Add SVG content to the document
    #[inline]
    pub fn add_svg<R>(&mut self,
                      svg_data: R)
    -> SvgIndex
    where R: ::std::io::Read
    {
        use lopdf::Object::*;
        use traits::IntoPdfObject;

        // todo
        let svg_obj = Svg::new(svg_data);
        let svg_obj_id = self.inner_doc.add_object(Box::new(svg_obj).into_obj());
        self.contents.push(Reference(svg_obj_id));
        SvgIndex(PdfContentIndex(self.contents.len() - 1))
    }

    // ----- GET FUNCTIONS

    /// Returns the page (for inserting content)
    #[inline]
    pub fn get_page_mut(&mut self, page: PdfPageIndex)
    -> &mut PdfPage
    {
        self.pages.get_mut(page.0).unwrap()
    }

    /// Drops the PDFDocument, returning the inner `lopdf::Document`. 
    /// Document may be only half-written
    #[inline]
    pub unsafe fn get_inner(self)
    -> (lopdf::Document, Vec<lopdf::Object>)
    {
        (self.inner_doc, self.contents)
    }

    // --- MISC FUNCTIONS

    /// Changes the title on both the document info dictionary as well as the metadata
    #[inline]
    pub fn set_title<S>(mut self, new_title: S)
    -> () where S: Into<String>
    {
        self.metadata.document_title = new_title.into();
    }

    /// Save PDF Document, writing the contents to the target
    pub fn save<W: Write + Seek>(mut self, target: &mut W)
    -> ::std::result::Result<(), Error>
    {
        use lopdf::{Dictionary as LoDictionary, 
                    Object as LoObject};
        use lopdf::Object::*;
        use std::iter::FromIterator;

        let pages_id = self.inner_doc.new_object_id();

        // extra pdf infos
        let (xmp_metadata, document_info, icc_profile) = self.metadata.into_obj();
        let xmp_metadata_id = self.inner_doc.add_object(xmp_metadata);
        let document_info_id = self.inner_doc.add_object(document_info);
            
        // add catalog 
        let icc_profile_descr = "Commercial and special offset print acccording to ISO \
                                 12647-2:2004 / Amd 1, paper type 1 or 2 (matte or gloss-coated \
                                 offset paper, 115 g/m2), screen ruling 60/cm";
        let icc_profile_str = "Coated FOGRA39 (ISO 12647-2:2004)";
        let icc_profile_short = "FOGRA39";

        use lopdf::StringFormat::Literal as Literal;
        let mut output_intents = LoDictionary::from_iter(vec![
                          ("S", Name("GTS_PDFX".into())),
                          ("OutputCondition", String(icc_profile_descr.into(), Literal)),
                          ("Type", Name("OutputIntent".into())),
                          ("OutputConditionIdentifier", String(icc_profile_short.into(), Literal)),
                          ("RegistryName", String("http://www.color.org".into(), Literal)),
                          ("Info", String(icc_profile_str.into(), Literal)), 
                          ]);

        // "Metadata" dictionary nicht komprimieren

        if let Some(profile) = icc_profile { 
            use traits::IntoPdfObject;
            let icc_profile_id = self.inner_doc.add_object(Box::new(profile).into_obj());
            output_intents.set("DestinationOutputProfile", Reference(icc_profile_id));
        }

        let catalog = LoDictionary::from_iter(vec![
                      ("Type", "Catalog".into()),
                      ("PageLayout", "OneColumn".into()),
                      ("PageMode", "Use0".into()),
                      ("Pages", Reference(pages_id)),
                      ("Metadata", Reference(xmp_metadata_id) ),
                      ("OutputIntents", Array(vec![Dictionary(output_intents)])),
                    ]);

        let mut pages = LoDictionary::from_iter(vec![
                      ("Type", "Pages".into()),
                      ("Count", Integer(self.pages.len() as i64)),
                      /* Kids and Resources missing */
                      ]);

        // add all pages with contents
        let mut page_ids = Vec::<LoObject>::new();

        for page in self.pages.into_iter() {
            
            let p = LoDictionary::from_iter(vec![
                      ("Type", "Page".into()),
                      ("Rotate", Integer(0)),
                      ("MediaBox", vec![0.into(), 0.into(),
                       page.width_pt.into(), page.heigth_pt.into()].into()),
                      ("TrimBox", vec![0.into(), 0.into(),
                       page.width_pt.into(), page.heigth_pt.into()].into()),
                      ("CropBox", vec![0.into(), 0.into(),
                       page.width_pt.into(), page.heigth_pt.into()].into()),
                      ("Parent", Reference(pages_id)) ]);

            // add page content (todo)

            page_ids.push(Reference(self.inner_doc.add_object(p)))
        }

        pages.set::<_, LoObject>("Kids".to_string(), page_ids.into());
        self.inner_doc.objects.insert(pages_id, Dictionary(pages));

        // save inner document
        let catalog_id = self.inner_doc.add_object(catalog);
        let instance_id: std::string::String = rand::thread_rng().gen_ascii_chars().take(32).collect();

        self.inner_doc.trailer.set("Root", Reference(catalog_id));
        self.inner_doc.trailer.set("Info", Reference(document_info_id));
        self.inner_doc.trailer.set("ID", Array(vec![
                                            String(self.document_id.as_bytes().to_vec(), Literal), 
                                            String(instance_id.as_bytes().to_vec(), Literal)
                                        ]));

        self.inner_doc.prune_objects();
        self.inner_doc.delete_zero_length_streams();
        // self.inner_doc.compress();
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