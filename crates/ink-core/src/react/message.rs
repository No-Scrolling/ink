use super::*;

#[derive(Deserialize)]
struct Picture { src: String, width: f32, height: f32 }
#[derive(Deserialize)]
struct Quote { author: String, text: String }
#[derive(Deserialize)]
struct Preview { domain: String, title: Option<String>, image: Option<Picture>, icon: Option<String>, thumbnail: Option<String> }
#[derive(Deserialize)]
struct Part { text: String, url: Option<String> }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    outgoing: bool,
    label: String,
    reactions: Option<String>,
    status_icon: Option<String>,
    status_muted: bool,
    reply: Option<Quote>,
    image: Option<Picture>,
    preview: Option<Preview>,
    parts: Vec<Part>,
}

impl ReactTree {
    pub(super) fn prepare_message(&mut self, id: usize, props: &Map<String, Json>) -> Result<()> {
        let count = 5 + props.get("parts").and_then(Json::as_array).map_or(0, Vec::len);
        ensure!(count <= 4096, "too many message parts");
        let mut ids = self.message_ids.remove(&id).unwrap_or_default();
        while ids.len() < count { ids.push(self.native_id()?); }
        self.message_ids.insert(id, ids);
        Ok(())
    }

    pub(super) fn render_message(&self, id: usize, host: &HostNode) -> Result<Option<Node>> {
        let props = &host.props;
        let HostProps::Other(data) = props else { bail!("invalid message properties"); };
        let message: Message = serde_json::from_value(Json::Object(data.clone()))?;
        let ids = self.message_ids.get(&id).context("message is not mounted")?;
        let target = |part: usize, name: &'static str, args: Vec<Json>| {
            let mut operation = event_operation(id, name, args);
            operation.event_target = Some((ids[part], name));
            Action::Native { operation: operation.into() }
        };
        let action = |part, name: &'static str| (props.get(name) == Some(&Json::Bool(true))).then(|| target(part, name, vec![]));
        let press = |child, action, long_action, haptic| Node { identity: NodeIdentity(0), kind: NodeKind::Pressable {
            children: vec![child], action, long_action, selected: false, haptic,
        } };
        let stack = |children, gap, axis, align| Node::stack(children, axis, Some(gap), align, Justification::Start);
        let text = |label: String, size, lines| Node::text(label.into(), Some(size), TextAlign::Start, lines, false);
        let picture = |picture: &Picture, fit| -> Result<Node> {
            ensure!(picture.width.is_finite() && picture.height.is_finite() && picture.width > 0.0
                && picture.height > 0.0 && picture.width <= 100_000.0 && picture.height <= 100_000.0, "invalid message image dimensions");
            let mut node = Node::image(image_source(&picture.src)?, None, false, false, picture.width, picture.height, fit);
            if let NodeKind::Image { looping, .. } = &mut node.kind { *looping = true; }
            Ok(node)
        };
        let mut labels = vec![text(message.label, 14.0, None)];
        if let Some(reactions) = message.reactions.filter(|value| !value.is_empty()) { labels.push(text(format!(", {reactions}"), 12.0, None)); }
        let mut metadata = vec![stack(labels, 0.0, Axis::Horizontal, Alignment::Centre)];
        if let Some(icon) = message.status_icon {
            metadata.push(Node::icon(self.icon(&icon)?, 14.0, if message.status_muted { Tone::Muted } else { Tone::Primary }));
        }
        let mut metadata = stack(metadata, 4.0, Axis::Horizontal, Alignment::Centre);
        if let Some(retry) = action(1, "onRetry") { metadata = press(metadata, Some(retry), None, true); }
        let metadata = stack(vec![metadata], 0.0, Axis::Vertical, if message.outgoing { Alignment::End } else { Alignment::Start });
        let mut content = Vec::new();
        if let Some(reply) = message.reply {
            let quoted = stack(vec![text(format!("Replying to {}", reply.author), 14.0, None), text(reply.text, 14.0, Some(1))],
                3.0, Axis::Vertical, Alignment::Start);
            content.push(press(Node { identity: NodeIdentity(0), kind: NodeKind::MessageQuote { children: vec![quoted] } }, action(3, "onReplyPress"), None, true));
        }
        if let Some(image) = message.image {
            let mut image = picture(&image, ImageFit::Contain)?;
            image.identity = NodeIdentity(ids[2]);
            content.push(press(image, action(2, "onImagePress"), action(0, "onLongPress"), true));
        }
        if !message.parts.is_empty() {
            let mut body = String::new();
            let mut links = Vec::new();
            for (index, part) in message.parts.into_iter().enumerate() {
                let start = body.len();
                body.push_str(&part.text);
                if let Some(url) = part.url { links.push((start..body.len(), target(5 + index, "onLinkPress", vec![json!(url)]))); }
            }
            let mut body = text(body, 20.0, None);
            if let NodeKind::Text { links: targets, .. } = &mut body.kind { *targets = links; }
            content.push(body);
        }
        if let Some(preview) = message.preview {
            let mut children = Vec::new();
            if let Some(image) = preview.image {
                ensure!(image.width > 0.0 && image.height > 0.0, "invalid preview dimensions");
                children.push(picture(&Picture { src: image.src, width: 220.0, height: 220.0 * image.height / image.width }, ImageFit::Cover)?);
            }
            let mut labels = Vec::new();
            if let Some(title) = preview.title.filter(|value| !value.is_empty()) { labels.push(text(title, 16.0, Some(2))); }
            let mut domain = Vec::new();
            if let Some(icon) = preview.icon.filter(|value| !value.is_empty()) {
                domain.push(picture(&Picture { src: icon, width: 16.0, height: 16.0 }, ImageFit::Contain)?);
            }
            domain.push(text(preview.domain, 13.0, Some(1)));
            labels.push(stack(domain, 8.0, Axis::Horizontal, Alignment::Centre));
            let labels = stack(labels, 4.0, Axis::Vertical, Alignment::Stretch);
            children.push(if let Some(src) = preview.thumbnail {
                stack(vec![picture(&Picture { src, width: 56.0, height: 56.0 }, ImageFit::Contain)?, labels],
                    10.0, Axis::Horizontal, Alignment::Centre)
            } else { labels });
            content.push(press(Node { identity: NodeIdentity(0), kind: NodeKind::LinkPreview { children } }, action(4, "onPreviewPress"), action(0, "onLongPress"), true));
        }
        let body = stack(vec![metadata, stack(content, 8.0, Axis::Vertical, Alignment::Stretch)], 4.0, Axis::Vertical, Alignment::Stretch);
        Ok(Some(Node { identity: NodeIdentity(id), kind: NodeKind::Message {
            outgoing: message.outgoing,
            children: vec![press(body, action(0, "onPress"), action(0, "onLongPress"), false)],
        } }))
    }
}
