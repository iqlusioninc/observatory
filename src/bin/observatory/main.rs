//! Main entry point for Observatory

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use observatory::application::APP;

/// Boot Observatory
fn main() {
    abscissa_core::boot(&APP);
}
