use super::*;

#[test]
fn fifteen_second_seek_icons_rasterise_with_the_original_ring_and_new_numerals() {
    for (fifteen, ten) in [("replay_15", "replay_10"), ("forward_15", "forward_10")] {
        let icon = raster_filled(fifteen, 56.0).unwrap();
        let original = raster_filled(ten, 56.0).unwrap();
        assert_eq!((icon.width, icon.height), (143, 143));
        assert!(icon.pixels.iter().any(|&pixel| pixel == 255));
        assert_ne!(icon.id, original.id);
        // The numerals sit below the upper ring and arrow. A missing glyph or
        // accidental use of the general filled font changes this region.
        let upper_ring = 40 * usize::from(icon.width);
        assert_eq!(icon.pixels[..upper_ring], original.pixels[..upper_ring]);
        assert_ne!(icon.pixels, original.pixels);
    }
}
