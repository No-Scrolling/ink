use super::*;

impl ReactTree {
    pub(super) fn render_row(&self, id: usize, props: &HostProps) -> Result<Node> {
        let title = string(props, "title").context("Row requires a title")?;
        let max_lines = props.get("titleMaxLines").filter(|value| !value.is_null()).map(|value| {
            value.as_u64().and_then(|value| u32::try_from(value).ok()).context("invalid titleMaxLines")
        }).transpose()?;
        let icon = |name: &str, size| -> Result<Node> {
            Ok(Node { identity: NodeIdentity(0), kind: NodeKind::Icon { mask: self.icon(name)?, size, tone: Tone::Primary, bounds: None } })
        };
        let mut content = vec![if let Some(name) = string(props, "titleIcon") {
            Node { identity: NodeIdentity(0), kind: NodeKind::RowTitle { text: title.into(), size: 26.0, max_lines,
                children: vec![icon(name, 26.0)?] } }
        } else { Node::text(title.into(), Some(26.0), TextAlign::Start, max_lines, false) }];
        let mut subtitle = Vec::new();
        if let Some(name) = string(props, "subtitleIcon") { subtitle.push(icon(name, 16.0)?); }
        if let Some(text) = string(props, "subtitle") { subtitle.push(Node::text(text.into(), Some(16.0), TextAlign::Start, Some(1), true)); }
        if !subtitle.is_empty() { content.push(Node::stack(subtitle, Axis::Horizontal, Some(6.0), Alignment::Centre, Justification::Start)); }
        let mut children = Vec::new();
        let image = string(props, "image");
        if let Some(src) = image { children.push(Node::image(image_source(src)?, None, false, false, 50.0, 50.0, ImageFit::Cover)); }
        children.push(Node::stack(content, Axis::Vertical, Some(0.0), Alignment::Start, Justification::Start));
        Ok(Node { identity: NodeIdentity(id), kind: NodeKind::Row { children, has_image: image.is_some(),
            action: (props.get("onPress") == Some(&Json::Bool(true))).then(|| event(id, "onPress", vec![])),
            long_action: (props.get("onLongPress") == Some(&Json::Bool(true))).then(|| event(id, "onLongPress", vec![])),
        } })
    }

    pub(super) fn render_avatar(&self, id: usize, props: &HostProps) -> Result<Node> {
        let src = string(props, "src").unwrap_or_default();
        let fallback = super::super::ImageAsset::new(u64::MAX - 1, 256, 256, include_bytes!("../avatar.rgba.zlib"));
        let source = if src.is_empty() { ImageSource::Asset(fallback.clone()) } else { image_source(src)? };
        let mut image = Node::image(source, Some(fallback), false, false, 72.0, 72.0, ImageFit::Cover);
        // Image retention and zoom state use the containing host's stable identity.
        image.identity = NodeIdentity(id);
        if let NodeKind::Image { avatar, .. } = &mut image.kind { *avatar = Some(props.get("unread") == Some(&Json::Bool(true))); }
        Ok(Node { identity: NodeIdentity(id), kind: NodeKind::Pressable { children: vec![image], selected: false, haptic: true,
            action: (props.get("onPress") == Some(&Json::Bool(true))).then(|| event(id, "onPress", vec![])),
            long_action: (props.get("onLongPress") == Some(&Json::Bool(true))).then(|| event(id, "onLongPress", vec![])),
        } })
    }
}
