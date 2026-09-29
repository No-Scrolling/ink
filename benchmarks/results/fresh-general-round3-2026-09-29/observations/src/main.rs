use ink_core::{Engine, ReactCommit, ReactTree};
use ink_protocol::Operation;
use serde_json::json;
fn main() {
    let template = json!({"id":20,"type":"Stack","props":{"gap":0},"children":[{"id":21,"type":"Text","props":{"text":{"$value":0,"path":["label"]}},"children":[]}]}).to_string();
    let definition = json!({"view":1,"values":[[2,true]],"root":{"id":3,"type":"Screen","props":{},"children":[{"id":4,"type":"NativeList","props":{"key":"id","items":[{"id":"a","label":"A"}],"hasMore":{"$value":2,"path":[]},"onEndReached":true,"template":template},"children":[]}]}}).to_string();
    let mut tree = ReactTree::default();
    let mut engine = Engine::new();
    engine.set_viewport(1080,1240);
    let initial = tree.apply(ReactCommit(vec![Operation::Create{id:5,r#type:ink_protocol::HostKind::NativeView,props:json!({"definition":definition}).as_object().unwrap().clone()},Operation::Insert{id:5,parent:0,before:None}]), &mut engine);
    println!("mount: {:?}", initial);
    let boundary = tree.viewport_events(&mut engine);
    println!("initial boundary: {:?}", boundary);
    let mut control_tree = tree.clone();
    let changed = tree.apply(ReactCommit(vec![Operation::Values{view:1,values:vec![(2,json!(false))],collections:vec![]}]), &mut engine);
    println!("change only bound hasMore: {:?}", changed);
    let control = control_tree.apply(ReactCommit(vec![Operation::Update{id:4,props:json!({"key":"id","hasMore":false,"onEndReached":true,"itemChanges":[]}).as_object().unwrap().clone()}]), &mut engine);
    println!("control includes explicit empty itemChanges: {:?}", control);
}
