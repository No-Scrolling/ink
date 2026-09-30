use super::*;

fn props(items: Vec<Json>) -> Map<String, Json> {
    json!({"key":"id", "items":items, "gap":12, "template":json!({
        "id":20,"type":"Stack","props":{"gap":0},"children":[
            {"id":21,"type":"Text","props":{"text":{"$value":0,"path":["label"]},"onPress":true},"children":[]}
        ]
    }).to_string()}).as_object().unwrap().clone()
}

fn fixture() -> (ReactTree, Engine, Vec<Json>) {
    let mut tree = ReactTree::default();
    let mut engine = Engine::new();
    engine.set_viewport(1080, 1240);
    let items: Vec<_> = (0..1000)
        .map(|i| json!({"id":format!("row-{i}"),"label":format!("Row {i}")}))
        .collect();
    tree.apply(
        ReactCommit(vec![
            Operation::Create {
                id: 1,
                r#type: HostKind::Screen,
                props: Map::new(),
            },
            Operation::Create {
                id: 2,
                r#type: HostKind::NativeList,
                props: props(items.clone()),
            },
            Operation::Insert {
                id: 2,
                parent: 1,
                before: None,
            },
            Operation::Insert {
                id: 1,
                parent: 0,
                before: None,
            },
        ]),
        &mut engine,
    )
    .unwrap();
    tree.refresh_native_lists(&mut engine).unwrap();
    (tree, engine, items)
}

#[test]
fn sparse_patch_preserves_key_storage_and_invalidates_only_changed_heights() {
    let (mut tree, mut engine, _) = fixture();
    let keys = tree.native_lists[&2].keys.clone();
    let versions = tree.native_lists[&2].versions.clone();
    let changed_label = "A much taller row ".repeat(80);
    tree.apply(
        ReactCommit(vec![Operation::Update {
            id: 2,
            props: json!({
                "key": "id", "gap": 12, "itemChanges": [{"id":"row-0", "label":changed_label}]
            })
            .as_object()
            .unwrap()
            .clone(),
        }]),
        &mut engine,
    )
    .unwrap();
    tree.refresh_native_lists(&mut engine).unwrap();
    assert!(Arc::ptr_eq(&keys, &tree.native_lists[&2].keys));
    assert_eq!(tree.native_lists[&2].versions[1..], versions[1..]);
    assert!(tree.native_lists[&2].versions[0] > versions[0]);
    assert!(
        engine
            .scene
            .text
            .iter()
            .any(|run| run.text.starts_with("A much taller row"))
    );
    assert!(
        engine.list_metrics[&2].offset(1)
            > engine.list_metrics[&2].offset(2) - engine.list_metrics[&2].offset(1)
    );
    engine.set_viewport(720, 1240);
    tree.refresh_native_lists(&mut engine).unwrap();
    engine.set_viewport(1080, 1240);
    tree.refresh_native_lists(&mut engine).unwrap();
    assert!(engine.list_viewports_ready());
}

#[test]
fn native_window_is_bounded_and_scroll_needs_no_javascript() {
    let (mut tree, mut engine, _) = fixture();
    for _ in 0..100 {
        engine.scroll_by(1200.0);
        assert!(tree.viewport_events(&mut engine).unwrap().is_empty());
        assert!(engine.list_viewports_ready());
        assert!(tree.native_lists[&2].rows.len() < 80);
        assert!(tree.nodes.len() < 170);
    }
    assert!(tree.native_lists[&2].start > 100);
    assert!(tree.native_events.len() < 160);
}

#[test]
fn keyed_updates_preserve_anchor_identity_and_route_row_actions() {
    let (mut tree, mut engine, mut items) = fixture();
    engine.scroll_by(20_000.0);
    tree.refresh_native_lists(&mut engine).unwrap();
    let metrics = &engine.list_metrics[&2];
    let top = engine.react_list_positions[&2];
    let index = metrics.index_at(engine.scroll_offset + engine.scene.scroll_clip.unwrap().y - top);
    let key = metrics.keys[index].clone();
    let old_y = top + metrics.offset(index) - engine.scroll_offset;
    let root = tree.native_lists[&2].rows[&key].root;
    let text = tree.native_lists[&2].rows[&key].nodes[1];
    let routed: Json = serde_json::from_str(
        &tree
            .route_native_message(
                json!({"type":"event","id":text,"name":"onPress","args":[]}).to_string(),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        routed,
        json!({"type":"event","id":21,"name":"onPress","args":[key]})
    );
    items.insert(0, json!({"id":"prepended","label":"Prepended"}));
    tree.apply(
        ReactCommit(vec![Operation::Update {
            id: 2,
            props: props(items.clone()),
        }]),
        &mut engine,
    )
    .unwrap();
    tree.refresh_native_lists(&mut engine).unwrap();
    assert_eq!(tree.native_lists[&2].rows[&key].root, root);
    let metrics = &engine.list_metrics[&2];
    let index = metrics.keys.iter().position(|item| item == &key).unwrap();
    let y = engine.react_list_positions[&2] + metrics.offset(index) - engine.scroll_offset;
    assert!((y - old_y).abs() < 1.0, "anchor moved: {old_y} to {y}");
    items.reverse();
    tree.apply(
        ReactCommit(vec![Operation::Update {
            id: 2,
            props: props(items.clone()),
        }]),
        &mut engine,
    )
    .unwrap();
    tree.refresh_native_lists(&mut engine).unwrap();
    assert_eq!(tree.native_lists[&2].rows[&key].root, root);
    items.retain(|item| item["id"] != key);
    tree.apply(
        ReactCommit(vec![Operation::Update {
            id: 2,
            props: props(items),
        }]),
        &mut engine,
    )
    .unwrap();
    tree.refresh_native_lists(&mut engine).unwrap();
    assert!(!tree.nodes.contains_key(&root));
    assert!(!tree.native_events.contains_key(&text));
    tree.apply(
        ReactCommit(vec![Operation::Remove { id: 1, parent: 0 }]),
        &mut engine,
    )
    .unwrap();
    assert!(tree.native_lists.is_empty());
    assert!(tree.native_events.is_empty());
}

#[test]
fn boundaries_request_once_and_empty_lists_release_rows() {
    let (mut tree, mut engine, items) = fixture();
    let mut properties = props(items.clone());
    properties.insert("hasOlder".into(), json!(true));
    properties.insert("onStartReached".into(), json!(true));
    properties.insert("onEndReached".into(), json!(true));
    properties.insert("followEnd".into(), json!(true));
    tree.apply(
        ReactCommit(vec![Operation::Update {
            id: 2,
            props: properties.clone(),
        }]),
        &mut engine,
    )
    .unwrap();
    let events = tree.viewport_events(&mut engine).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["name"], "onStartReached");
    assert!(tree.viewport_events(&mut engine).unwrap().is_empty());
    engine.scroll_by(f32::MAX);
    let events = tree.viewport_events(&mut engine).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["name"], "onEndReached");
    assert!(tree.viewport_events(&mut engine).unwrap().is_empty());
    properties["items"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"last","label":"Last"}));
    tree.apply(
        ReactCommit(vec![Operation::Update {
            id: 2,
            props: properties.clone(),
        }]),
        &mut engine,
    )
    .unwrap();
    let events = tree.viewport_events(&mut engine).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["name"], "onEndReached");
    assert!((engine.scroll_offset - engine.scroll_max).abs() < 1.0);
    properties.insert("items".into(), json!([]));
    tree.apply(
        ReactCommit(vec![Operation::Update {
            id: 2,
            props: properties,
        }]),
        &mut engine,
    )
    .unwrap();
    assert!(tree.viewport_events(&mut engine).unwrap().is_empty());
    assert!(tree.native_lists[&2].rows.is_empty());
    assert!(tree.native_events.is_empty());
}
