use anyhow::{Context, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::ir::{Action, App, Node, TextPart};

pub fn generate(app: &App) -> Result<String> {
    let states = app.states.iter().map(|state| {
        let initial = state.initial;
        quote! { StateValue::Int(#initial) }
    });
    let mut emitter = Emitter::default();
    let root = emitter.node(&app.root);
    let nodes = emitter.nodes;
    let tokens = quote! {
        use ink_core::{Action, AppDefinition, Node, StateId, StateValue, Style, TextPart};

        #[rustfmt::skip]
        pub fn app() -> ink_core::AppDefinition {
            #(#nodes)*
            AppDefinition::new(vec![#(#states),*], #root)
        }
    };
    let syntax = syn::parse2::<syn::File>(tokens).context("Ink generated invalid Rust")?;
    Ok(prettyplease::unparse(&syntax))
}

#[derive(Default)]
struct Emitter {
    next_node: usize,
    nodes: Vec<TokenStream>,
}

impl Emitter {
    fn node(&mut self, value: &Node) -> TokenStream {
        let value = match value {
            Node::Column { children, gap } => {
                let children = children.iter().map(|child| self.node(child));
                let column = quote! { Node::column(vec![#(#children),*]) };
                style(column, "gap", *gap)
            }
            Node::Text { parts, font_size } => {
                let parts = parts.iter().map(text_part);
                let text = quote! { Node::text(vec![#(#parts),*]) };
                style(text, "font_size", *font_size)
            }
            Node::Button { label, action } => {
                let action = action_tokens(action);
                quote! { Node::button(#label, #action) }
            }
        };
        let name = format_ident!("node_{}", self.next_node);
        self.next_node += 1;
        self.nodes.push(quote! { let #name = #value; });
        quote! { #name }
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
            quote! {
                Action::Increment {
                    state: StateId::new(#id),
                    by: #by,
                }
            }
        }
    }
}

fn style(node: TokenStream, field: &str, value: Option<f32>) -> TokenStream {
    let Some(value) = value else {
        return node;
    };

    match field {
        "gap" => quote! {
            #node.with_style(Style {
                gap: Some(#value),
                ..Default::default()
            })
        },
        "font_size" => quote! {
            #node.with_style(Style {
                font_size: Some(#value),
                ..Default::default()
            })
        },
        _ => unreachable!("unsupported generated style field"),
    }
}
