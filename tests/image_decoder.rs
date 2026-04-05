//! Integration tests for `image::ImageDecoder` implementation.
//!
//! These tests require the `image` feature flag to be enabled.

#[cfg(feature = "image")]
mod image_tests {
    use std::path::PathBuf;

    use djvu_rs::{DjVuDocument, image_compat::DjVuDecoder};
    use image::ImageDecoder;

    fn chicken_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/djvujs/library/assets/chicken.djvu")
    }

    fn chicken_doc() -> DjVuDocument {
        let data = std::fs::read(chicken_path()).expect("chicken.djvu must exist");
        DjVuDocument::parse(&data).expect("parse must succeed")
    }

    #[test]
    fn decoder_dimensions_match_page_info() {
        let doc = chicken_doc();
        let page = doc.page(0).expect("page 0 must exist");
        let decoder = DjVuDecoder::new(page).expect("decoder should construct");

        let (w, h) = decoder.dimensions();
        assert_eq!(w, 181);
        assert_eq!(h, 240);
    }

    #[test]
    fn decoder_color_type_is_rgba8() {
        let doc = chicken_doc();
        let page = doc.page(0).expect("page 0 must exist");
        let decoder = DjVuDecoder::new(page).expect("decoder should construct");

        assert_eq!(decoder.color_type(), image::ColorType::Rgba8);
    }

    #[test]
    fn decoder_total_bytes() {
        let doc = chicken_doc();
        let page = doc.page(0).expect("page 0 must exist");
        let decoder = DjVuDecoder::new(page).expect("decoder should construct");

        let (w, h) = decoder.dimensions();
        let expected = u64::from(w) * u64::from(h) * 4; // 4 bytes per RGBA pixel
        assert_eq!(decoder.total_bytes(), expected);
    }

    #[test]
    fn read_image_produces_correct_pixel_count() {
        let doc = chicken_doc();
        let page = doc.page(0).expect("page 0 must exist");
        let decoder = DjVuDecoder::new(page).expect("decoder should construct");

        let total = decoder.total_bytes() as usize;
        let mut buf = vec![0u8; total];
        let decoder2 = DjVuDecoder::new(doc.page(0).expect("page 0")).expect("decoder2");
        decoder2
            .read_image(&mut buf)
            .expect("read_image should succeed");

        // Verify at least some non-zero pixels (not a blank image)
        assert!(
            buf.iter().any(|&b| b != 0),
            "image should have non-zero pixels"
        );
    }

    #[test]
    fn decoder_with_size_overrides_dimensions() {
        let doc = chicken_doc();
        let page = doc.page(0).expect("page 0 must exist");
        let decoder = DjVuDecoder::new(page).expect("decoder").with_size(90, 120);

        let (w, h) = decoder.dimensions();
        assert_eq!(w, 90);
        assert_eq!(h, 120);
    }

    #[test]
    fn read_image_pixel_match_with_render_pixmap() {
        use djvu_rs::djvu_render::RenderOptions;

        let data = std::fs::read(chicken_path()).expect("chicken.djvu must exist");
        let doc1 = DjVuDocument::parse(&data).expect("parse");
        let doc2 = DjVuDocument::parse(&data).expect("parse");

        let page1 = doc1.page(0).expect("page 0");
        let page2 = doc2.page(0).expect("page 0");

        // Render via render_into
        let opts = RenderOptions {
            width: page1.width() as u32,
            height: page1.height() as u32,
            ..RenderOptions::default()
        };
        let expected_bytes = (opts.width * opts.height * 4) as usize;
        let mut direct_buf = vec![0u8; expected_bytes];
        page1
            .render_into(&opts, &mut direct_buf)
            .expect("render_into");

        // Render via ImageDecoder
        let decoder = DjVuDecoder::new(page2).expect("decoder");
        let total = decoder.total_bytes() as usize;
        let mut decoder_buf = vec![0u8; total];
        decoder.read_image(&mut decoder_buf).expect("read_image");

        assert_eq!(
            direct_buf, decoder_buf,
            "ImageDecoder output must match direct render_into output"
        );
    }
}
