use ink_core::{ImageUpdate, NativeRequestKind};
use ink_test::SceneApp;
use std::collections::HashMap;

#[test]
fn screen_reveal_waits_only_for_visible_images_while_overscan_images_keep_loading() {
    let mut app = SceneApp::new("images");
    app.pump_until(|app| app.labels().contains(&"Images".into()));
    let requests: HashMap<_, _> = std::iter::from_fn(|| app.engine.take_native_request())
        .map(|request| {
            assert_eq!(request.kind(), NativeRequestKind::Image);
            let url = serde_json::from_str::<serde_json::Value>(request.payload()).unwrap()["url"]
                .as_str().unwrap().to_owned();
            (url, request.id())
        })
        .collect();
    assert_eq!(requests.len(), 4, "images below the viewport must still be prefetched");
    assert!(!app.engine.screen_images_ready());

    assert_eq!(
        app.engine.complete_native_image(requests["https://example.com/visible.png"], 1, 1, vec![0; 4], None),
        ImageUpdate::Visible
    );
    assert!(app.engine.screen_images_ready(), "offscreen images must not delay the screen");
    assert!(
        app.engine.take_native_request().is_none(),
        "offscreen requests must not be cancelled"
    );
    for name in ["below-1", "below-2", "below-3"] {
        let id = requests[&format!("https://example.com/{name}.png")];
        assert_eq!(
            app.engine.complete_native_image(id, 1, 1, vec![0; 4], None),
            ImageUpdate::Offscreen,
            "an offscreen image must not be presented as a visible change"
        );
    }
    assert_eq!(app.engine.scene().images.len(), 4, "offscreen images must be ready for scrolling");
}
