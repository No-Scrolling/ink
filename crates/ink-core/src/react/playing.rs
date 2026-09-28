use super::*;

impl ReactTree {
    pub(super) fn prepare_playing(&mut self, id: usize, props: &Map<String, Json>) -> Result<()> {
        let count = 9 + props.get("artists").and_then(Json::as_array).map_or(0, Vec::len)
            + props.get("actions").and_then(Json::as_array).map_or(0, Vec::len);
        ensure!(count <= 4096, "too many player controls");
        let mut ids = self.playing_ids.remove(&id).unwrap_or_default();
        while ids.len() < count { ids.push(self.native_id()?); }
        self.playing_ids.insert(id, ids);
        Ok(())
    }

    pub(super) fn render_playing(&self, id: usize, host: &HostNode) -> Result<Option<Node>> {
        let props = &host.props;
        let ids = self.playing_ids.get(&id).context("player is not mounted")?;
        let action = |part: usize, name: &'static str, args: Vec<Json>| {
            let mut operation = event_operation(id, name, args);
            operation.event_target = Some((ids[part], name));
            Action::Native { operation: operation.into() }
        };
        let pressable = |part, child: Node, action, long_action, selected| Node {
            identity: NodeIdentity(ids[part]), kind: NodeKind::Pressable {
                children: vec![child], action, long_action, selected, haptic: true,
            },
        };
        let icon = |name: &str, size, muted| -> Result<Node> {
            let mask = self.icon(name)?;
            let (key, filled) = icon_reference(name);
            let variants = self.icons.get(key).context("missing icon variants")?;
            let cache = if filled { &variants.filled_bounds } else { &variants.outlined_bounds };
            let bounds = *cache.get_or_init(|| mask.content_bounds());
            Ok(Node { identity: NodeIdentity(0), kind: NodeKind::Icon {
                mask, size, tone: if muted { Tone::Muted } else { Tone::Primary }, bounds,
            } })
        };
        let vertical = |children, gap, align| Node::stack(children, Axis::Vertical, Some(gap), align, Justification::Start);
        let mut content = Vec::new();
        let image = string(props, "image").filter(|image| !image.is_empty());
        if let Some(image) = image {
            let mut node = Node::image(image_source(image)?, None, false, false, 200.0, 200.0, ImageFit::Cover);
            node.identity = NodeIdentity(ids[8]);
            if let NodeKind::Image { preload, retain_while_loading, .. } = &mut node.kind {
                *retain_while_loading = true;
                if let Some(images) = props.get("preloadImages").and_then(Json::as_array) {
                    *preload = images.iter().take(2).map(|image| image_source(image.as_str().context("invalid preload image")?)).collect::<Result<_>>()?;
                }
            }
            content.push(vertical(vec![node], 0.0, Alignment::Centre));
        }
        let mut labels = vec![pressable(0, Node { identity: NodeIdentity(ids[4]), kind: NodeKind::PlayingLabel {
            text: string(props, "title").unwrap_or_default().to_owned(), size: 22.0,
        } }, (props.get("onTitlePress") == Some(&Json::Bool(true))).then(|| action(0, "onTitlePress", vec![])), None, false)];
        let artists = props.get("artists").and_then(Json::as_array).context("player requires artists")?;
        for (index, artist) in artists.iter().enumerate() {
            let part = 9 + index;
            labels.push(pressable(part, Node { identity: NodeIdentity(ids[part]), kind: NodeKind::PlayingLabel {
                text: artist.get("name").and_then(Json::as_str).context("artist requires a name")?.to_owned(), size: 14.0,
            } }, (artist.get("interactive") == Some(&Json::Bool(true))).then(|| action(part, "onArtistPress", vec![json!(index)])), None, false));
        }
        content.push(vertical(labels, 0.0, Alignment::Centre));
        let position = playback_seconds(props, "position", 1000.0)?;
        let duration = playback_seconds(props, "duration", 1000.0)?;
        let playing = props.get("playing") == Some(&Json::Bool(true));
        let loading = props.get("loading") == Some(&Json::Bool(true));
        content.push(Node { identity: NodeIdentity(id), kind: NodeKind::PlayingProgress {
            position: position.min(duration), duration,
            clock: props.get("clock").and_then(Json::as_u64), speed: playback_speed(props)?,
            playing: playing && !loading && props.get("buffering") != Some(&Json::Bool(true)),
            seek: props.get("onSeek") == Some(&Json::Bool(true)), show_times: true,
        } });
        let control = |part, prefix: &str, press, long_press| -> Result<Node> {
            let disabled = props.get(&format!("{prefix}Disabled")) == Some(&Json::Bool(true));
            Ok(pressable(part, icon(string(props, &format!("{prefix}Icon")).context("missing transport icon")?, 56.0, disabled)?,
                (!disabled).then(|| action(part, press, vec![])),
                (!disabled && props.get(long_press) == Some(&Json::Bool(true))).then(|| action(part, long_press, vec![])), false))
        };
        let transport = vec![
            control(1, "previous", "onPreviousPress", "onPreviousLongPress")?,
            pressable(2, icon(string(props, if playing { "pauseIcon" } else { "playIcon" }).context("missing play icon")?, 56.0, false)?, Some(action(2, "onPlayPause", vec![])), None, false),
            control(3, "next", "onNextPress", "onNextLongPress")?,
        ];
        content.push(Node { identity: NodeIdentity(ids[5]), kind: NodeKind::PlayingTransport { children: transport, loading } });
        let mut actions = Vec::new();
        if let Some(items) = props.get("actions").and_then(Json::as_array) {
            for (index, item) in items.iter().enumerate() {
                let part = 9 + artists.len() + index;
                let child = if let Some(label) = item.get("label").and_then(Json::as_str) {
                    Node::text(label.into(), Some(22.0), TextAlign::Start, None, true)
                } else { icon(item.get("icon").and_then(Json::as_str).context("action requires a label or icon")?, 44.0, false)? };
                actions.push(pressable(part, child, (item.get("disabled") != Some(&Json::Bool(true))).then(|| action(part, "onActionPress", vec![json!(index)])), None,
                    item.get("selected") == Some(&Json::Bool(true))));
            }
        }
        let layout = Node { identity: NodeIdentity(ids[6]), kind: NodeKind::PlayingLayout {
            children: vec![vertical(content, 16.0, Alignment::Stretch), Node { identity: NodeIdentity(ids[7]), kind: NodeKind::PlayingTransport { children: actions, loading: false } }],
            centred: image.is_none(), hide_controls: false, bleed: false,
        } };
        let mut screen = Node::screen(vec![layout], None, false);
        screen.identity = NodeIdentity(id);
        Ok(Some(screen))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_clock_updates_time_speed_pause_and_seek_without_a_commit() {
        let mut engine = Engine::new(); engine.set_viewport(1080, 1240);
        let mut tree = ReactTree::default();
        tree.apply(ReactCommit(vec![
            Operation::Create { id: 1, r#type: HostKind::Screen, props: Map::new() },
            Operation::Create { id: 2, r#type: HostKind::PlayingProgress, props: json!({
                "position":0,"duration":120,"playing":false,"showTimes":true,"clock":7,"onSeek":true
            }).as_object().unwrap().clone() },
            Operation::Insert { id:2,parent:1,before:None }, Operation::Insert { id:1,parent:0,before:None },
        ]), &mut engine).unwrap();
        assert!(engine.update_playback_clock(7, 10.0, 120.0, true, 2.0));
        engine.playback_progress.get_mut(&NodeIdentity(2)).unwrap().started = std::time::Instant::now() - std::time::Duration::from_secs(3);
        engine.animate_scene();
        assert!(engine.scene.text.iter().any(|run| run.text == "0:16"));
        engine.update_playback_clock(7, 35.0, 120.0, false, 1.0);
        engine.animate_scene();
        assert!(engine.scene.text.iter().any(|run| run.text == "0:35"));
        assert!(!engine.playback_progress[&NodeIdentity(2)].animating());
        engine.update_playback_clock(7, 9.0, 200.0, false, 1.0);
        engine.animate_scene();
        assert!(engine.scene.text.iter().any(|run| run.text == "3:20"));
        assert!(engine.scene.text.iter().any(|run| run.text == "0:09"));
        engine.remove_playback_clock(7);
        assert!(engine.playback_clocks.is_empty());
    }

    #[test]
    fn playback_times_are_not_layout_dimensions() {
        for (milliseconds, seconds) in [(213_000, 213.0), (180_000_000, 180_000.0), (0, 0.0)] {
            let props = HostProps::Other(Map::from_iter([("duration".into(), json!(milliseconds))]));
            assert_eq!(playback_seconds(&props, "duration", 1000.0).unwrap(), seconds);
        }
        for value in [json!(-1), Json::Null, json!("213000"), json!(f64::MAX)] {
            let props = HostProps::Other(Map::from_iter([("duration".into(), value)]));
            assert!(playback_seconds(&props, "duration", 1000.0).is_err());
        }
    }
}
