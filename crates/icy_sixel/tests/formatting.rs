use icy_sixel::{SixelError, SixelImage};
use std::fmt::{self, Write};

#[test]
fn display_preserves_successful_sixel_encoding() {
    let image = SixelImage::try_from_rgba(vec![255, 0, 0, 255], 1, 1).unwrap();
    assert_eq!(image.to_string(), image.encode().unwrap());
}

#[test]
fn display_reports_encoding_failures_without_panicking() {
    for image in [
        SixelImage::from_rgba(vec![], 0, 0),
        SixelImage::from_rgba(vec![], 1, 1),
        SixelImage::from_rgba(vec![], usize::MAX, 2),
        SixelImage::try_from_rgba(vec![0; 1_000_001 * 4], 1_000_001, 1).unwrap(),
    ] {
        let expected = format!("[SIXEL encoding failed: {}]", image.encode().unwrap_err());
        assert_eq!(image.to_string(), expected);
        assert_eq!(format!("image={image}"), format!("image={expected}"));
    }
}

#[test]
fn display_still_propagates_writer_failures() {
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }
    for image in [SixelImage::from_rgba(vec![255; 4], 1, 1), SixelImage::from_rgba(vec![], 0, 0)] {
        assert!(write!(&mut FailingWriter, "{image}").is_err());
    }
}

#[test]
fn explicit_encoding_keeps_structured_errors() {
    let image = SixelImage::from_rgba(vec![], 0, 0);
    assert!(matches!(image.encode(), Err(SixelError::InvalidDimensions { .. })));
}
