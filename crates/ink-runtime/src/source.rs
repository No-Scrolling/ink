use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use ink_protocol::{
    HostKind, Operation, ReactCommit,
    source::{Graph, Record, ViewChange, ViewSnapshot},
};
use rquickjs::{Ctx, Exception, FromJs, Function, IntoJs, Object, Result, Value};
use serde::Deserialize;
use serde_json::{Map, Value as Json, json};

use crate::{EventSink, commit};

pub(crate) type Shared = Rc<RefCell<Sources>>;

#[derive(Default)]
pub(crate) struct Sources {
    graph: Option<Graph>,
    hosts: HashMap<usize, Host>,
    bindings: HashMap<u64, HashSet<usize>>,
}

#[derive(Default)]
struct Host {
    native_list: bool,
    parent: Option<usize>,
    children: Vec<usize>,
    view: Option<u64>,
}

impl Sources {
    fn snapshot(&self, view: u64) -> std::result::Result<Arc<ViewSnapshot>, String> {
        self.graph
            .as_ref()
            .ok_or_else(|| "unknown source view".to_owned())?
            .snapshot(view)
    }

    // Keep topology even before the first source: a later binding can sit beneath
    // an ancestor that was mounted before sources were used.
    pub(crate) fn commit(
        &mut self,
        ctx: &Ctx<'_>,
        batch: &mut ReactCommit,
        bytes: &mut usize,
    ) -> Result<()> {
        let mut snapshots = HashMap::new();
        let mut native_lists = HashSet::new();
        for operation in &batch.0 {
            let (props, native_list) = match operation {
                Operation::Create { id, r#type, props } => {
                    let native_list = *r#type == HostKind::NativeList;
                    if native_list {
                        native_lists.insert(*id);
                    }
                    (props, native_list)
                }
                Operation::Update { id, props } => (
                    props,
                    native_lists.contains(id)
                        || self.hosts.get(id).is_some_and(|host| host.native_list),
                ),
                _ => continue,
            };
            if let Some(value) = props.get("sourceView") {
                if !native_list {
                    return Err(Exception::throw_type(
                        ctx,
                        "sourceView requires a NativeList",
                    ));
                }
                let view =
                    identifier(value).map_err(|message| Exception::throw_type(ctx, message))?;
                let snapshot = self
                    .snapshot(view)
                    .map_err(|message| Exception::throw_type(ctx, &message))?;
                snapshots.insert(view, snapshot);
                *bytes = bytes.saturating_add(64);
                commit::check_size(ctx, *bytes)?;
            }
        }

        let mut initial = HashMap::new();
        for operation in &batch.0 {
            match operation {
                Operation::Create { id, r#type, props } => {
                    self.remove(*id, &mut initial);
                    self.hosts.insert(
                        *id,
                        Host {
                            native_list: *r#type == HostKind::NativeList,
                            ..Host::default()
                        },
                    );
                    self.bind(*id, props, &snapshots, &mut initial);
                }
                Operation::Update { id, props } => self.bind(*id, props, &snapshots, &mut initial),
                Operation::Insert { id, parent, .. } => {
                    if let Some(previous) = self.hosts.get(id).and_then(|host| host.parent)
                        && let Some(host) = self.hosts.get_mut(&previous)
                    {
                        host.children.retain(|child| child != id);
                    }
                    self.hosts.entry(*id).or_default().parent = Some(*parent);
                    self.hosts.entry(*parent).or_default().children.push(*id);
                }
                Operation::Remove { id, .. } => self.remove(*id, &mut initial),
                _ => {}
            }
        }
        batch
            .0
            .extend(initial.into_iter().map(|(id, snapshot)| Operation::Source {
                id,
                snapshot,
                previous_revision: 0,
                changed_keys: Vec::new(),
                structural: true,
            }));
        Ok(())
    }

    fn bind(
        &mut self,
        id: usize,
        props: &Map<String, Json>,
        snapshots: &HashMap<u64, Arc<ViewSnapshot>>,
        initial: &mut HashMap<usize, Arc<ViewSnapshot>>,
    ) {
        let Some(host) = self.hosts.get(&id) else {
            return;
        };
        if !host.native_list {
            return;
        }
        let view = match props.get("sourceView") {
            Some(value) => value.as_u64(),
            None if props.contains_key("items") || props.contains_key("itemChanges") => None,
            None => return,
        };
        if host.view == view {
            return;
        }
        self.unbind(id);
        initial.remove(&id);
        if let Some(view) = view {
            self.hosts.get_mut(&id).unwrap().view = Some(view);
            self.bindings.entry(view).or_default().insert(id);
            initial.insert(id, snapshots[&view].clone());
        }
    }

    fn unbind(&mut self, id: usize) {
        if let Some(view) = self.hosts.get_mut(&id).and_then(|host| host.view.take())
            && let Some(hosts) = self.bindings.get_mut(&view)
        {
            hosts.remove(&id);
            if hosts.is_empty() {
                self.bindings.remove(&view);
            }
        }
    }

    fn remove(&mut self, id: usize, initial: &mut HashMap<usize, Arc<ViewSnapshot>>) {
        if !self.hosts.contains_key(&id) {
            return;
        }
        if let Some(parent) = self.hosts.get(&id).and_then(|host| host.parent)
            && let Some(host) = self.hosts.get_mut(&parent)
        {
            host.children.retain(|child| *child != id);
        }
        let mut pending = vec![id];
        while let Some(id) = pending.pop() {
            self.unbind(id);
            initial.remove(&id);
            if let Some(host) = self.hosts.remove(&id) {
                pending.extend(host.children);
            }
        }
    }

    fn release(&mut self, id: u64) -> std::result::Result<(), String> {
        let graph = self
            .graph
            .as_mut()
            .ok_or_else(|| "unknown source view".to_owned())?;
        graph.release(id)?;
        self.bindings.retain(|view, hosts| {
            if graph.snapshot(*view).is_ok() {
                return true;
            }
            for id in hosts.iter() {
                if let Some(host) = self.hosts.get_mut(id) {
                    host.view = None;
                }
            }
            false
        });
        Ok(())
    }

    fn publish(
        &self,
        ctx: &Ctx<'_>,
        changes: &[ViewChange],
        payload_bytes: usize,
        events: &EventSink,
        failed: &Rc<RefCell<bool>>,
    ) -> Result<()> {
        let mut operations = Vec::new();
        let mut bytes = payload_bytes;
        for change in changes {
            let Some(hosts) = self.bindings.get(&change.view) else {
                continue;
            };
            let metadata = change.changed_keys.iter().fold(64usize, |bytes, key| {
                bytes.saturating_add(24).saturating_add(key.len())
            });
            for &id in hosts {
                bytes = bytes.saturating_add(metadata);
                operations.push(Operation::Source {
                    id,
                    snapshot: change.snapshot.clone(),
                    previous_revision: change.previous_revision,
                    changed_keys: change.changed_keys.clone(),
                    structural: change.structural,
                });
            }
        }
        if operations.is_empty() {
            return Ok(());
        }
        commit::send(ctx, events, failed, ReactCommit(operations), bytes)
    }
}

pub(crate) fn install<'js>(
    ctx: &Ctx<'js>,
    sources: Shared,
    events: EventSink,
    failed: Rc<RefCell<bool>>,
) -> Result<()> {
    let snapshots = sources.clone();
    let snapshot_failed = failed.clone();
    ctx.globals().set(
        "__inkSourceSnapshot",
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, view: Identifier| {
            commit::ensure_open(&ctx, &snapshot_failed)?;
            let snapshot = snapshots
                .borrow()
                .snapshot(view.0)
                .map_err(|message| Exception::throw_type(&ctx, &message))?;
            snapshot_object(ctx, snapshot)
        })?,
    )?;
    ctx.globals().set(
        "__inkSource",
        Function::new(
            ctx.clone(),
            move |ctx: Ctx<'_>, operation: String, payload: String| {
                commit::ensure_open(&ctx, &failed)?;
                let mut sources = sources.borrow_mut();
                let response = match operation.as_str() {
                    "create" => {
                        let Create { key, items } = decode(&ctx, &payload)?;
                        let graph = sources.graph.get_or_insert_with(Graph::default);
                        let id = graph
                            .create(key, items)
                            .map_err(|message| Exception::throw_type(&ctx, &message))?;
                        let snapshot = graph
                            .snapshot(id)
                            .map_err(|message| Exception::throw_type(&ctx, &message))?;
                        descriptor(id, &snapshot)
                    }
                    "view" => {
                        let View { source, r#where } = decode(&ctx, &payload)?;
                        let graph = sources
                            .graph
                            .as_mut()
                            .ok_or_else(|| Exception::throw_type(&ctx, "unknown source"))?;
                        let id = graph
                            .view(source, r#where)
                            .map_err(|message| Exception::throw_type(&ctx, &message))?;
                        let snapshot = graph
                            .snapshot(id)
                            .map_err(|message| Exception::throw_type(&ctx, &message))?;
                        descriptor(id, &snapshot)
                    }
                    "set" | "replace" => {
                        let Mutation {
                            source,
                            base_revision,
                            items,
                        } = decode(&ctx, &payload)?;
                        let graph = sources
                            .graph
                            .as_mut()
                            .ok_or_else(|| Exception::throw_type(&ctx, "unknown source"))?;
                        let changes = if operation == "set" {
                            graph.set(source, base_revision, items)
                        } else {
                            graph.replace(source, base_revision, items)
                        }
                        .map_err(|message| Exception::throw_type(&ctx, &message))?;
                        let revision = graph
                            .snapshot(source)
                            .map_err(|message| Exception::throw_type(&ctx, &message))?
                            .revision;
                        let views: Vec<_> = changes
                            .iter()
                            .map(|change| descriptor(change.view, &change.snapshot))
                            .collect();
                        sources.publish(&ctx, &changes, payload.len(), &events, &failed)?;
                        json!({ "revision": revision, "views": views })
                    }
                    "release" => {
                        let Release { id } = decode(&ctx, &payload)?;
                        sources
                            .release(id)
                            .map_err(|message| Exception::throw_type(&ctx, &message))?;
                        Json::Null
                    }
                    _ => {
                        return Err(Exception::throw_type(
                            &ctx,
                            "unknown native source operation",
                        ));
                    }
                };
                Ok(response.to_string())
            },
        )?,
    )
}

fn snapshot_object<'js>(ctx: Ctx<'js>, snapshot: Arc<ViewSnapshot>) -> Result<Object<'js>> {
    let object = Object::new(ctx.clone())?;
    object.set("revision", snapshot.revision as f64)?;
    object.set("count", snapshot.len() as f64)?;
    let get = snapshot.clone();
    object.set(
        "get",
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, key: String| {
            record_json(&ctx, get.get(&key))
        })?,
    )?;
    object.set(
        "at",
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, index: Value<'js>| {
            let record = index
                .as_number()
                .filter(|number| {
                    number.fract() == 0.0 && *number >= 0.0 && *number < snapshot.len() as f64
                })
                .and_then(|index| snapshot.at(index as usize));
            record_json(&ctx, record)
        })?,
    )?;
    if unsafe { rquickjs::qjs::JS_FreezeObject(ctx.as_raw().as_ptr(), object.as_raw()) } < 0 {
        return Err(rquickjs::Error::Exception);
    }
    Ok(object)
}

fn record_json<'js>(ctx: &Ctx<'js>, record: Option<&Record>) -> Result<Value<'js>> {
    match record {
        Some(record) => record.value.to_string().into_js(ctx),
        None => Ok(Value::new_null(ctx.clone())),
    }
}

fn descriptor(id: u64, snapshot: &ViewSnapshot) -> Json {
    json!({ "id": id, "revision": snapshot.revision, "count": snapshot.len() })
}

fn decode<T: serde::de::DeserializeOwned>(ctx: &Ctx<'_>, payload: &str) -> Result<T> {
    serde_json::from_str(payload).map_err(|error| Exception::throw_type(ctx, &error.to_string()))
}

fn identifier(value: &Json) -> std::result::Result<u64, &'static str> {
    value
        .as_u64()
        .filter(|id| *id <= 9_007_199_254_740_991)
        .ok_or("invalid source view identifier")
}

struct Identifier(u64);

impl<'js> FromJs<'js> for Identifier {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        value
            .as_number()
            .filter(|number| {
                number.fract() == 0.0 && (0.0..=9_007_199_254_740_991.0).contains(number)
            })
            .map(|number| Self(number as u64))
            .ok_or_else(|| Exception::throw_type(ctx, "invalid source view identifier"))
    }
}

#[derive(Deserialize)]
struct Create {
    key: String,
    items: Vec<Json>,
}

#[derive(Deserialize)]
struct View {
    source: u64,
    r#where: Map<String, Json>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mutation {
    source: u64,
    base_revision: u64,
    items: Vec<Json>,
}

#[derive(Deserialize)]
struct Release {
    id: u64,
}
