use std::{alloc::Layout, collections::HashMap};
use syn::{
    parse::Error, parse_macro_input, parse_quote, punctuated::Punctuated, Fields, Item, Path,
    TypePath,
};

mod parsed {
    #[derive(Debug)]
    pub enum Struct {
        Unit,
        Tuple(Vec<syn::Type>),
        Struct(Vec<(syn::Ident, syn::Type)>),
    }

    #[derive(Debug)]
    pub enum Item {
        Struct(Struct),
        Enum(Vec<(syn::Ident, Struct)>),
        Union(Vec<(syn::Ident, syn::Type)>),
        TypeAlias(syn::Type),
    }

    #[derive(Clone, Debug, Eq, PartialEq, Hash)]
    pub struct TypePath(pub syn::TypePath);

    impl TypePath {
        pub fn new() -> Self {
            Self(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments: syn::punctuated::Punctuated::new(),
                },
            })
        }

        pub fn is_absolute(&self) -> bool {
            if self.0.path.leading_colon.is_none() {
                assert!(self.0.qself.is_none());
            }
            self.0.path.leading_colon.is_some()
        }

        pub fn concat(&self, other: TypePath) -> Self {
            if other.is_absolute() {
                other
            } else {
                let mut segments = self.0.path.segments.clone();
                segments.push_punct(<syn::Token![::]>::default());
                segments.extend(other.0.path.segments);
                TypePath(syn::TypePath {
                    qself: self.0.qself.clone(),
                    path: syn::Path {
                        leading_colon: self.0.path.leading_colon,
                        segments,
                    },
                })
            }
        }

        pub fn push(&mut self, item: syn::PathSegment) {
            self.0.path.segments.push(item)
        }
    }
}

macro_rules! impl_add_builtins {
    ($self:ident; $($type:ty)*) => {
        $($self.processed_items.insert(parsed::TypePath(parse_quote!($type)), Layout::new::<$type>());)*
    }
}

fn parse_struct_fields(fields: Fields) -> parsed::Struct {
    match fields {
        Fields::Named(x) => parsed::Struct::Struct(
            x.named
                .into_iter()
                .map(|y| (y.ident.unwrap(), y.ty))
                .collect(),
        ),
        #[rustfmt::skip]
        Fields::Unnamed(x) => parsed::Struct::Tuple(
            x.unnamed
                .into_iter()
                .map(|y| y.ty)
                .collect()
        ),
        Fields::Unit => parsed::Struct::Unit,
    }
}

#[derive(Debug)]
struct Data {
    unprocessed_items: HashMap<parsed::TypePath, parsed::Item>,
    processed_items: HashMap<parsed::TypePath, Layout>,
}

impl Data {
    pub fn new() -> Self {
        let mut self_ = Self {
            unprocessed_items: HashMap::new(),
            processed_items: HashMap::new(),
        };
        self_.add_builtins();
        self_
    }

    fn add_builtins(&mut self) {
        impl_add_builtins! { self; u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }
    }

    pub fn add_item(&mut self, parent_path: parsed::TypePath, item: Item) -> Result<(), Error> {
        let (ident, parsed_item) = match item {
            Item::Mod(x) => {
                if let Some((_, items)) = x.content {
                    let ident = x.ident;
                    let mut path: parsed::TypePath = parent_path;
                    path.push(ident.into());
                    for sub_item in items {
                        self.add_item(path.clone(), sub_item)?;
                    }
                } else {
                    return Err(Error::new_spanned(
                        x,
                        "pahole does not currently support `mod`s implemented in other files",
                    ));
                }
                return Ok(());
            }
            Item::Enum(x) => (
                x.ident,
                parsed::Item::Enum(
                    x.variants
                        .into_iter()
                        .map(|y| (y.ident, parse_struct_fields(y.fields)))
                        .collect(),
                ),
            ),
            Item::Struct(x) => (x.ident, parsed::Item::Struct(parse_struct_fields(x.fields))),
            Item::Type(x) => (x.ident, parsed::Item::TypeAlias(*x.ty)),
            Item::Union(x) => (
                x.ident,
                parsed::Item::Union(
                    x.fields
                        .named
                        .into_iter()
                        .map(|y| (y.ident.unwrap(), y.ty))
                        .collect(),
                ),
            ),
            _ => {
                return Err(Error::new_spanned(item, "pahole can currently only process `mod`s, `enum`s, `struct`s, `type`s, and `union`s."));
            }
        };
        let mut path = parent_path;
        path.push(ident.into());
        self.unprocessed_items.insert(path, parsed_item);
        Ok(())
    }
}

#[proc_macro_attribute]
pub fn pahole(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_cloned = item.clone();
    let syn_item = parse_macro_input!(item as Item);
    let mut data = Data::new();
    match data.add_item(parsed::TypePath::new(), syn_item) {
        Ok(()) => {}
        Err(err) => return err.to_compile_error().into(),
    }
    dbg!(data);
    item_cloned
}
