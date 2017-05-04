//! A marker is a position on a page on a layer inside a pdf document

use super::*;

/// Postion on the page from the top right corner
#[derive(Debug, Copy, Clone)]
pub struct PdfMarker {
    /// Horizontal postion in point
    pub x_pt: f64,
    /// Horizontal postion in point
    pub y_pt: f64,
}

impl PdfMarker {
    
    /// Create a new marker, notice that x and y are in millimeters
    pub fn new(x_mm: f64, y_mm: f64)
    -> Self 
    {
        Self {
            x_pt: mm_to_pt!(x_mm),
            y_pt: mm_to_pt!(y_mm),
        }
    }
}