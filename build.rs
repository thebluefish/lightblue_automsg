use cargo_emit::warning;
use inflector::Inflector;
use path_slash::*;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::{path, path::PathBuf};
use indexmap::IndexMap;
use syn::punctuated::Punctuated;
use syn::parse::Parser;
use syn::{Item, Meta, Token, Path, parse_quote, MetaNameValue, ItemEnum, ItemStruct, Attribute, MetaList, Expr, ExprLit, Lit};
use walkdir::WalkDir;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=src");

    let cwd = env!("CARGO_MANIFEST_DIR");

    // parse every rust file in /src for items relevant to our protocols
    // sort all items into a bucket for each protocol
    let mut protocols = parse_dir(path::Path::new(cwd).join("src"))?;

    // generate missing items for each protocol
    let gen_root: Path = parse_quote!(crate::gen);
    let mut items = Vec::<TokenStream>::new();
    for (_, protocol) in &mut protocols {
        if protocol.component_protocol.is_none() {
            items.push(protocol.gen_component_protocol(&gen_root));
        }
        if protocol.message_protocol.is_none() {
            items.push(protocol.gen_message_protocol(&gen_root));
        }
        if protocol.input_protocol.is_none() {
            items.push(protocol.gen_input(&gen_root));
        }
        items.push(protocol.gen_protocolize());
    }

    // build final output
    let out = quote!(#(#items)*);

    // write output to shared path
    let dest_path = PathBuf::from(env::var("OUT_DIR")?).join("gen.rs");
    fs::write(&dest_path, out.to_string())?;

    Ok(())
}

#[derive(Debug)]
struct Protocol {
    name: Ident,
    protocolize: Option<ProtocolItem>,
    component_protocol: Option<ProtocolItem>,
    message_protocol: Option<ProtocolItem>,
    input_protocol: Option<ProtocolItem>,
    components: Vec<ProtocolItem>,
    messages: Vec<ProtocolItem>,
    inputs: Vec<ProtocolItem>,
}

impl Protocol {
    /// whether this protocol is completely defined by the user or not
    fn is_complete(&self) -> bool {
        self.protocolize.is_some() && self.component_protocol.is_some() && self.message_protocol.is_some()
    }

    /// checks that we can generate the missing pieces
    fn generatable(&self) -> bool {
        !self.is_complete() && self.protocolize.is_none()
    }

    /// generates the component_protocol sum type, adds its entry to the protocol, and returns its fragment to be included in the final output
    fn gen_component_protocol(&mut self, root: &Path) -> TokenStream {
        let protocol_str = self.name.to_string();
        let struct_ident = Ident::new(&format!("{}Components", protocol_str.to_pascal_case()), self.name.span());
        self.component_protocol = Some(ProtocolItem {
            kind: ItemKind::ComponentProtocols,
            path: parse_quote!(#root :: #struct_ident),
        });
        let fragments = self.components.iter().map(|item| {
            let path = &item.path;
            let sync = {
                let ItemKind::Component { sync } = &item.kind else { unreachable!() };
                if let Some(sync) = sync {
                    quote!(#sync)
                }
                else {
                    quote!(sync(simple))
                }
            };
            let name = &path.segments.last().unwrap().ident;
            quote!(
                #[#sync]
                #name(#path)
            )
        }).collect::<Vec<_>>();
        quote!(
            #[lightyear::prelude::component_protocol(protocol = #protocol_str)]
            pub enum #struct_ident {
                #(#fragments,)*
            }
        )
    }

    /// generates the component_protocol sum type, adds its entry to the protocol, and returns its fragment to be included in the final output
    fn gen_message_protocol(&mut self, root: &Path) -> TokenStream {
        let protocol_str = self.name.to_string();
        let struct_ident = Ident::new(&format!("{}Messages", protocol_str.to_pascal_case()), self.name.span());
        self.message_protocol = Some(ProtocolItem {
            kind: ItemKind::MessageProtocols,
            path: parse_quote!(#root :: #struct_ident),
        });
        let fragments = self.messages.iter().map(|item| {
            let path = &item.path;
            let name = &path.segments.last().unwrap().ident;
            quote!(#name(#path))
        }).collect::<Vec<_>>();
        quote!(
            #[lightyear::prelude::message_protocol(protocol = #protocol_str)]
            pub enum #struct_ident {
                #(#fragments,)*
            }
        )
    }

    /// generates the Input map
    fn gen_input(&mut self, _root: &Path) -> TokenStream {
        todo!("this is left as an exercise for the reader
How you generate your input map depends on whether you want to map unit structs to unit variants or single-field tuple variants")
    }

    /// generates the final protocolize definition
    fn gen_protocolize(&mut self) -> TokenStream {
        let protocol = &self.name;
        let message = &self.message_protocol.as_ref().unwrap().path;
        let message_item = &message.segments.last().unwrap().ident;
        let component = &self.component_protocol.as_ref().unwrap().path;
        let mut component_kind = component.clone();
        {
            let last = component_kind.segments.last_mut().unwrap();
            last.ident = Ident::new(&format!("{}Kind", last.ident.to_string()), last.ident.span());
        }
        let component_item = &component.segments.last().unwrap().ident;
        let input = &self.input_protocol.as_ref().unwrap().path;
        let input_item = &input.segments.last().unwrap().ident;
        // stuff into a module so we can freely `use` imports to satisfy macro constraints
        quote!(
            pub mod protocol {
                use lightyear::protocolize;
                use #message;
                use #component;
                use #component_kind;
                use #input;
                protocolize! {
                    Self = #protocol,
                    Message = #message_item,
                    Component = #component_item,
                    Input = #input_item,
                }
            }
            pub use protocol :: *;
        )
    }
}

/// find all protocolize! calls
/// find all component_protocol and message_protocol items
/// find all component, message, and input items
fn parse_dir<P: AsRef<path::Path>>(path: P) -> anyhow::Result<IndexMap<Ident, Protocol>> {
    let mut parsed = Vec::new();

    // Find all rust files in the directory
    for file in WalkDir::new(&path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| !p.is_dir() && p.extension().map(|ext| ext.eq_ignore_ascii_case("rs")).unwrap_or(false))
    {
        // parse all items in the file and add them to the list
        parsed.extend(parse_file(&path, &file)?);
    }

    // sort our items into each protocol being defined
    let mut protocols: IndexMap<Ident, Protocol> = IndexMap::new();
    for item in parsed {
        if !protocols.contains_key(&item.protocol) {
            protocols.insert(item.protocol.clone(), Protocol {
                name: item.protocol.clone(),
                protocolize: None,
                component_protocol: None,
                message_protocol: None,
                input_protocol: None,
                components: vec![],
                messages: vec![],
                inputs: vec![],
            });
        }
        let protocol = protocols.get_mut(&item.protocol).unwrap();
        match &item.kind {
            ItemKind::Protocolize => {
                match protocol.protocolize {
                    None => protocol.protocolize = Some(item.into()),
                    Some(_) => panic!("multiple protocolize! definitions found for the same protocol")
                }
            }
            ItemKind::ComponentProtocols => {
                match protocol.component_protocol {
                    None => protocol.component_protocol = Some(item.into()),
                    Some(_) => panic!("multiple #[component_protocol] definitions found for the same protocol")
                }
            }
            ItemKind::MessageProtocols => {
                match protocol.message_protocol {
                    None => protocol.message_protocol = Some(item.into()),
                    Some(_) => panic!("multiple #[message_protocol] definitions found for the same protocol")
                }
            }
            ItemKind::InputProtocols => {
                match protocol.input_protocol {
                    None => protocol.input_protocol = Some(item.into()),
                    Some(_) => panic!("multiple #[inputs] definitions found for the same protocol")
                }
            }
            ItemKind::Component{..} => protocol.components.push(item.into()),
            ItemKind::Message => protocol.messages.push(item.into()),
            ItemKind::Input => protocol.inputs.push(item.into()),
        }
    }


    // only pass up the protocols we can do something with
    protocols.retain(|_, protocol| protocol.generatable());

    Ok(protocols)
}

fn parse_file<R: AsRef<path::Path>, P: AsRef<path::Path>>(root: R, path: P) -> anyhow::Result<Vec<ParsedItem>> {
    let mut items = Vec::new();

    let mut file = File::open(&path).unwrap();

    // generate a module path based on the file's location relative to the root dir
    // eg. "src/player/net/mod.rs" becomes "crate::player::net"
    // ideally we attempt to find a better public path, for example to support "mod foo; pub use foo;"
    // but this is an exercise for whoever needs a more flexible project structure
    let path = path.as_ref().strip_prefix(&root).unwrap();
    let fqp = path.with_extension("");
    let fqp = fqp.into_iter()
        .map(|s| s.to_string_lossy().to_lowercase())
        .filter(|s| !["main", "lib", "mod"].contains(&&**s))
        .map(|s| Ident::new(&s, Span::call_site()))
        .collect::<Vec<_>>();
    let fqp: Path = parse_quote!(crate #(:: #fqp )*);

    let mut src = String::new();
    file.read_to_string(&mut src).unwrap();
    let syntax = syn::parse_file(&src);

    match syntax {
        Ok(file) => {
            for item in file.items {
                items.extend(parse_item(fqp.clone(), item));
            }
        },
        Err(err) => {
            warning!("skipping {}: {}", path.to_slash_lossy(), err);
        },
    };

    Ok(items)
}

#[derive(Debug)]
enum ItemKind {
    Protocolize,
    ComponentProtocols,
    MessageProtocols,
    InputProtocols,
    Component {
        sync: Option<MetaList>,
    },
    Message,
    Input,
}

struct ParsedItem {
    kind: ItemKind,
    path: Path,
    protocol: Ident,
}

#[derive(Debug)]
struct ProtocolItem {
    kind: ItemKind,
    path: Path,
}

impl From<ParsedItem> for ProtocolItem {
    fn from(value: ParsedItem) -> Self {
        ProtocolItem {
            kind: value.kind,
            path: value.path,
        }
    }
}

/// Parses item into zero or more items
fn parse_item(root: Path, item: Item) -> Vec<ParsedItem> {
    let mut items = Vec::new();

    // attempt to pull out relevant info for struct & enum items
    let (ident, attrs) = match item {
        // inline modules need to be parsed recursively
        Item::Mod(item) => {
            if let Some((_, content)) = item.content {
                let name = &item.ident;
                let root: Path = parse_quote!(#root :: #name);

                for item in content {
                    items.extend(parse_item(root.clone(), item));
                }
            }
            return items
        }
        // attempt to parse the `protocolize!` macro into its major components
        Item::Macro(item) => {
            let name = &item.mac.path.segments.last().unwrap().ident;
            if name == "protocolize" {
                let mut protocol: Option<Ident> = None;
                // parse macro content as a list of "name = value" pairs
                for MetaNameValue { path, value, .. } in Punctuated::<MetaNameValue, Token![,]>::parse_terminated.parse2(item.mac.tokens).unwrap() {
                    match &*path.segments.last().unwrap().ident.to_string() {
                        "Self" => protocol = Some(parse_quote!(#value)),
                        "Message" | "Component" | "Input" => {},
                        _ => {} // todo: simplify this? no longer parsing everything out
                    }
                }

                let protocol = protocol.expect("Protocol must be defined with a `Self = Name`");
                let path: Path = parse_quote!(#root :: #protocol);

                items.push(ParsedItem {
                    kind: ItemKind::Protocolize,
                    path,
                    protocol,
                })
            }
            return items
        }
        Item::Enum(ItemEnum { ident, attrs, .. }) => (ident, attrs),
        Item::Struct(ItemStruct { ident, attrs, .. }) => (ident, attrs),
        _ => return items
    };

    let mut kind: Option<String> = None;
    let mut protocol: Option<Ident> = None;
    let mut sync: Option<MetaList> = None;

    for Attribute { meta, .. } in attrs {
        match meta {
            Meta::List(MetaList { path, tokens, .. }) => {
                let pname = path.segments.last().unwrap().ident.to_string();
                if matches!(&*pname, "component" | "message" | "input" | "component_protocol" | "message_protocol" | "inputs") {
                    kind = Some(pname);
                    for attr in Punctuated::<Meta, Token![,]>::parse_terminated.parse2(tokens).unwrap() {
                        match attr {
                            Meta::Path(path) => {
                                protocol = Some(path.segments.into_iter().last().unwrap().ident);
                            }
                            Meta::List(item) => {
                                if item.path.is_ident("sync") {
                                    sync = Some(item);
                                }
                            }
                            Meta::NameValue(MetaNameValue { path, value, .. }) => {
                                if path.is_ident("protocol") {
                                    protocol = Some(value.try_to_ident().unwrap());
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let kind: Option<ItemKind> = kind.map(|k| Some(match &*k {
        "component" => ItemKind::Component {
            sync,
        },
        "message" => ItemKind::Message,
        "input" => ItemKind::Input,
        "component_protocol" => ItemKind::ComponentProtocols,
        "message_protocol" => ItemKind::MessageProtocols,
        "inputs" => ItemKind::InputProtocols,
        _ => return None
    })).flatten();

    if let (Some(kind), Some(protocol)) = (kind, protocol) {
        let path: Path = parse_quote!(#root :: #ident);
        items.push(ParsedItem {
            kind,
            path,
            protocol,
        })
    }

    items
}

/// copied from bevy_commandify
pub trait ExprExt {
    fn try_to_path(&self) -> Option<Path>;
    fn try_to_ident(&self) -> Option<Ident>;
}

impl ExprExt for Expr {
    fn try_to_path(&self) -> Option<Path> {
        match &self {
            Expr::Lit(ExprLit {
                          lit: Lit::Str(lit), ..
                      }) => lit.parse_with(Path::parse_mod_style).ok(),
            Expr::Path(path) => Some(path.path.clone()),
            _ => None
        }
    }

    fn try_to_ident(&self) -> Option<Ident> {
        match &self {
            Expr::Lit(ExprLit {
                          lit: Lit::Str(lit), ..
                      }) => lit.parse().ok(),
            Expr::Path(path) => {
                if path.path.segments.is_empty() {
                    return None
                }
                if path.path.segments.len() > 1 {
                    return None
                }
                Some(path.path.clone().segments.pop().unwrap().into_value().ident)
            }
            _ => None
        }
    }
}