use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    icons,
    ir::{
        Action, Alignment, App, Axis, ImageFit, Justification, Node, StateValue, TextAlignment,
        TextPart, Tone,
    },
};

const DEFAULT_ICON_SIZE: f32 = 28.0;
const BUTTON_ICON_SIZE: f32 = 30.0;
const TAB_ICON_SIZE: f32 = 48.0;
const TOGGLE_ICON_SIZE: f32 = 9.8;

pub fn generate(app: &App, root: &Path) -> Result<String> {
    let states = app.states.iter().map(|state| state_value(state.initial));
    let mut emitter = Emitter::new(root);
    let root = emitter.node(&app.root)?;
    let declarations = emitter.declarations;
    let tokens = quote! {
        use ink_core::{
            Action, Alignment, AppDefinition, Axis, Justification, Mask, Node, StateId, StateValue,
            Tab, TextAlign, TextPart, Tone,
        };

        #[rustfmt::skip]
        pub fn app() -> ink_core::AppDefinition {
            #(#declarations)*
            AppDefinition::new(vec![#(#states),*], #root)
        }
    };
    let syntax = syn::parse2::<syn::File>(tokens).context("Ink generated invalid Rust")?;
    Ok(prettyplease::unparse(&syntax))
}

struct Emitter<'a> {
    root: &'a Path,
    next_node: usize,
    next_mask: usize,
    next_asset: usize,
    declarations: Vec<TokenStream>,
    masks: HashMap<(String, u32), TokenStream>,
    assets: HashMap<PathBuf, TokenStream>,
}

impl<'a> Emitter<'a> {
    fn new(root: &'a Path) -> Self {
        Self {
            root,
            next_node: 0,
            next_mask: 0,
            next_asset: 0,
            declarations: Vec::new(),
            masks: HashMap::new(),
            assets: HashMap::new(),
        }
    }

    fn node(&mut self, value: &Node) -> Result<TokenStream> {
        let value = match value {
            Node::Screen {
                children,
                title,
                centered,
            } => {
                let children = self.children(children)?;
                let title = option_string(title.as_deref());
                quote! { Node::screen(vec![#(#children),*], #title, #centered) }
            }
            Node::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            } => {
                let children = self.children(children)?;
                let axis = axis_tokens(*axis);
                let gap = option_f32(*gap);
                let align = alignment_tokens(*align);
                let justify = justification_tokens(*justify);
                quote! { Node::stack(vec![#(#children),*], #axis, #gap, #align, #justify) }
            }
            Node::Text {
                parts,
                font_size,
                align,
            } => {
                let parts = parts.iter().map(text_part);
                let font_size = option_f32(*font_size);
                let align = text_alignment_tokens(*align);
                quote! { Node::text(vec![#(#parts),*], #font_size, #align) }
            }
            Node::TextInput { placeholder } => quote! { Node::text_input(#placeholder) },
            Node::Button {
                label,
                icon,
                underline,
                action,
            } => {
                let icon = match icon {
                    Some(name) => {
                        let mask = self.mask(name, BUTTON_ICON_SIZE)?;
                        quote! { Some(#mask) }
                    }
                    None => quote! { None },
                };
                let action = match action {
                    Some(action) => {
                        let action = action_tokens(action);
                        quote! { Some(#action) }
                    }
                    None => quote! { None },
                };
                quote! { Node::button(#label, #icon, #underline, #action) }
            }
            Node::Icon { name, size, tone } => {
                let size = size.unwrap_or(DEFAULT_ICON_SIZE);
                let mask = self.mask(name, size)?;
                let tone = tone_tokens(*tone);
                quote! { Node::icon(#mask, #size, #tone) }
            }
            Node::Image {
                source,
                width,
                height,
                fit,
            } => {
                let asset = self.asset(source)?;
                let fit = image_fit_tokens(*fit);
                quote! { Node::image(#asset, #width, #height, #fit) }
            }
            Node::Toggle {
                label,
                state,
                action,
            } => {
                let off = self.toggle_mask(false);
                let on = self.toggle_mask(true);
                let state = state.0;
                let action = action_tokens(action);
                quote! {
                    Node::toggle(
                        #label,
                        StateId::new(#state),
                        #action,
                        #off,
                        #on,
                    )
                }
            }
            Node::Tabs { state, tabs } => {
                let mut generated_tabs = Vec::with_capacity(tabs.len());
                for tab in tabs {
                    let icon = self.mask(&tab.icon, TAB_ICON_SIZE)?;
                    let action = action_tokens(&tab.action);
                    let screen = self.node(&tab.screen)?;
                    generated_tabs.push(quote! { Tab::new(#icon, #action, #screen) });
                }
                let state = state.0;
                quote! { Node::tabs(StateId::new(#state), vec![#(#generated_tabs),*]) }
            }
        };

        let name = format_ident!("node_{}", self.next_node);
        self.next_node += 1;
        self.declarations.push(quote! { let #name = #value; });
        Ok(quote! { #name })
    }

    fn children(&mut self, children: &[Node]) -> Result<Vec<TokenStream>> {
        children.iter().map(|child| self.node(child)).collect()
    }

    fn mask(&mut self, name: &str, size: f32) -> Result<TokenStream> {
        let key = (name.to_owned(), size.to_bits());
        if let Some(mask) = self.masks.get(&key) {
            return Ok(mask.clone());
        }

        let icon = icons::raster(name, size)?;
        let id = icon.id;
        let width = icon.width;
        let height = icon.height;
        let pixels = icon.pixels;
        let name = format_ident!("mask_{}", self.next_mask);
        self.next_mask += 1;
        self.declarations.push(quote! {
            let #name = Mask::new(#id, #width, #height, &[#(#pixels),*]);
        });
        let mask = quote! { #name };
        self.masks.insert(key, mask.clone());
        Ok(mask)
    }

    fn toggle_mask(&mut self, filled: bool) -> TokenStream {
        let key = (
            if filled {
                "ink_toggle_on"
            } else {
                "ink_toggle_off"
            }
            .to_owned(),
            TOGGLE_ICON_SIZE.to_bits(),
        );
        if let Some(mask) = self.masks.get(&key) {
            return mask.clone();
        }

        let icon = icons::toggle_circle(filled);
        let id = icon.id;
        let width = icon.width;
        let height = icon.height;
        let pixels = icon.pixels;
        let name = format_ident!("mask_{}", self.next_mask);
        self.next_mask += 1;
        self.declarations.push(quote! {
            let #name = Mask::new(#id, #width, #height, &[#(#pixels),*]);
        });
        let mask = quote! { #name };
        self.masks.insert(key, mask.clone());
        mask
    }

    fn asset(&mut self, source: &str) -> Result<TokenStream> {
        let path = self
            .root
            .join(source)
            .canonicalize()
            .with_context(|| format!("could not find image {source:?}"))?;
        if !path.starts_with(self.root) {
            anyhow::bail!("image {source:?} must be inside the Ink application");
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("png") {
            anyhow::bail!("image {source:?} must be a PNG file");
        }
        if let Some(asset) = self.assets.get(&path) {
            return Ok(asset.clone());
        }

        let bytes = std::fs::read(&path)
            .with_context(|| format!("could not read image {}", path.display()))?;
        let pixels = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .with_context(|| format!("could not decode image {}", path.display()))?
            .into_rgba8();
        let width = pixels.width();
        let height = pixels.height();
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(pixels.as_raw(), 9);
        let id = hash_bytes(&bytes);
        let name = format_ident!("image_{}", self.next_asset);
        self.next_asset += 1;
        self.declarations.push(quote! {
            let #name = ink_core::ImageAsset::new(#id, #width, #height, &[#(#compressed),*]);
        });
        let asset = quote! { #name };
        self.assets.insert(path, asset.clone());
        Ok(asset)
    }
}

fn state_value(value: StateValue) -> TokenStream {
    match value {
        StateValue::Int(value) => quote! { StateValue::Int(#value) },
        StateValue::Bool(value) => quote! { StateValue::Bool(#value) },
    }
}

fn text_part(part: &TextPart) -> TokenStream {
    match part {
        TextPart::Literal(value) => quote! { TextPart::literal(#value) },
        TextPart::State(state) => {
            let id = state.0;
            quote! { TextPart::state(StateId::new(#id)) }
        }
    }
}

fn action_tokens(action: &Action) -> TokenStream {
    match action {
        Action::Increment { state, by } => {
            let id = state.0;
            quote! { Action::Increment { state: StateId::new(#id), by: #by } }
        }
        Action::SetInt { state, value } => {
            let id = state.0;
            quote! { Action::SetInt { state: StateId::new(#id), value: #value } }
        }
        Action::SetBool { state, value } => {
            let id = state.0;
            quote! { Action::SetBool { state: StateId::new(#id), value: #value } }
        }
        Action::Toggle { state } => {
            let id = state.0;
            quote! { Action::Toggle { state: StateId::new(#id) } }
        }
    }
}

fn axis_tokens(axis: Axis) -> TokenStream {
    match axis {
        Axis::Vertical => quote! { Axis::Vertical },
        Axis::Horizontal => quote! { Axis::Horizontal },
    }
}

fn alignment_tokens(alignment: Alignment) -> TokenStream {
    match alignment {
        Alignment::Start => quote! { Alignment::Start },
        Alignment::Center => quote! { Alignment::Centre },
        Alignment::End => quote! { Alignment::End },
        Alignment::Stretch => quote! { Alignment::Stretch },
    }
}

fn justification_tokens(justification: Justification) -> TokenStream {
    match justification {
        Justification::Start => quote! { Justification::Start },
        Justification::Center => quote! { Justification::Centre },
        Justification::End => quote! { Justification::End },
        Justification::SpaceBetween => quote! { Justification::SpaceBetween },
    }
}

fn text_alignment_tokens(alignment: TextAlignment) -> TokenStream {
    match alignment {
        TextAlignment::Start => quote! { TextAlign::Start },
        TextAlignment::Center => quote! { TextAlign::Centre },
        TextAlignment::End => quote! { TextAlign::End },
    }
}

fn tone_tokens(tone: Tone) -> TokenStream {
    match tone {
        Tone::Primary => quote! { Tone::Primary },
        Tone::Muted => quote! { Tone::Muted },
    }
}

fn image_fit_tokens(fit: ImageFit) -> TokenStream {
    match fit {
        ImageFit::Cover => quote! { ink_core::ImageFit::Cover },
        ImageFit::Contain => quote! { ink_core::ImageFit::Contain },
    }
}

fn option_f32(value: Option<f32>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value) },
        None => quote! { None },
    }
}

fn option_string(value: Option<&str>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value.to_owned()) },
        None => quote! { None },
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut value = 0xcbf29ce484222325_u64;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(0x100000001b3);
    }
    value
}
