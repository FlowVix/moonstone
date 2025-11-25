use std::time::Instant;

use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    AngleBracketedGenericArguments, Expr, Ident, Pat, Path, Token, Type, Visibility, braced,
    bracketed, parenthesized, parse::Parse, parse_quote, punctuated::Punctuated, token,
};

mod kw {
    syn::custom_keyword!(view);
}

pub struct ViewDef {
    vis: Visibility,
    base: Type,
    typ: ViewType,
}
pub enum ViewType {
    Struct {
        name: Ident,
        body: Punctuated<ViewField, Token![,]>,
    },
}

pub struct ViewField {
    vis: Visibility,
    view: Option<kw::view>,
    name: Ident,
    typ: Type,
    body: Option<Punctuated<ViewField, Token![,]>>,
}

impl Parse for ViewDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let vis = input.parse()?;
        if input.peek(Token![struct]) {
            input.parse::<Token![struct]>()?;
            let name = input.parse()?;
            input.parse::<Token![:]>()?;
            let base = input.parse()?;
            let inner;
            braced!(inner in input);
            let body = Punctuated::parse_terminated(&inner)?;
            Ok(ViewDef {
                vis,
                base,
                typ: ViewType::Struct { name, body },
            })
        } else {
            panic!("Struct or enum bro")
        }
    }
}

impl Parse for ViewField {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let vis = input.parse()?;
        let view = if input.peek(kw::view) {
            Some(input.parse::<kw::view>()?)
        } else {
            None
        };
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let typ = input.parse()?;
        let body = if input.peek(token::Brace) {
            let inner;
            braced!(inner in input);
            Some(Punctuated::parse_terminated(&inner)?)
        } else {
            None
        };
        Ok(ViewField {
            vis,
            view,
            name,
            typ,
            body,
        })
    }
}

struct DataCollect {
    pub init_struct_fields: TokenStream,
    pub view_struct_fields: TokenStream,
    pub build_view_values: TokenStream,
    pub build_fields: TokenStream,
}

fn collect_data(list: &Punctuated<ViewField, Token![,]>, data: &mut DataCollect) {
    for field in list {
        let vis = &field.vis;
        let name = &field.name;

        let typ = &field.typ;
        match (field.view, &field.body) {
            (Some(v), None) => {
                let kw = Ident::new("try", v.span);
                // if *name != "__" {
                data.init_struct_fields.extend(quote! { #vis #name: #typ, });
                data.view_struct_fields
                    .extend(quote! { #vis #name: ::moonstone::ViewValue<#typ>, });

                data.build_view_values.extend(quote! {
                    stringify!(#kw);
                    let __state = <#typ as ::moonstone::View>::build(&self.#name, __parent.clone().upcast(), ::moonstone::AnchorType::ChildOf);
                    let #name = ::moonstone::ViewValue::create(self.#name, __state);
                });
                data.build_fields.extend(quote! {
                    #name,
                });
                // }
            }
            (None, None) => {
                // if *name != "__" {
                data.init_struct_fields.extend(quote! { #vis #name: #typ, });
                data.view_struct_fields.extend(quote! { #vis #name: #typ, });
                data.build_fields.extend(quote! {
                    #name: self.#name,
                });
                // }
            }
            (_, Some(body)) => {
                // if *name != "__" {
                data.init_struct_fields
                    .extend(quote! { #vis #name: ::godot::obj::Gd<#typ>, });
                data.view_struct_fields
                    .extend(quote! { #vis #name: ::godot::obj::Gd<#typ>, });
                // }

                data.build_view_values.extend(quote! {
                    __parent.add_child(&self.#name);
                    let mut __parent = self.#name.clone();
                });
                // if *name != "__" {
                data.build_fields.extend(quote! {
                    #name: self.#name,
                });
                // }

                collect_data(body, data);

                data.build_view_values.extend(quote! {
                    let mut __parent = __parent.get_parent().unwrap();
                });
            }
        };
    }
}

impl ViewDef {
    pub fn gen_rust(&self) -> TokenStream {
        let base_type = &self.base;
        let vis = &self.vis;
        match &self.typ {
            ViewType::Struct { name, body } => {
                let mut collect = DataCollect {
                    init_struct_fields: quote! {},
                    view_struct_fields: quote! {},
                    build_view_values: quote! {},
                    build_fields: quote! {},
                };

                collect_data(body, &mut collect);

                let mod_name = format_ident!("_def_{}", name);
                let init_struct_name = format_ident!("{}_Init", name);
                let DataCollect {
                    init_struct_fields,
                    view_struct_fields,
                    build_view_values,
                    build_fields,
                } = collect;

                quote! {

                    #[derive(::godot::prelude::GodotClass)]
                    #[class(base=#base_type, no_init)]
                    #vis struct #name {
                        base: ::godot::obj::Base<#base_type>,
                        #view_struct_fields
                    }
                    #[allow(non_camel_case_types)]
                    #vis struct #init_struct_name {
                        #init_struct_fields
                    }

                    #[doc(hidden)]
                    #[allow(non_snake_case)]
                    mod #mod_name {
                        use super::*;
                        use std::rc::{Rc, Weak};
                        use std::cell::{RefCell, Ref, RefMut};


                        impl #init_struct_name {
                            pub fn build(self) -> ::godot::obj::Gd<#name> {
                                use ::godot::obj::NewAlloc;
                                let out = ::godot::obj::Gd::from_init_fn(|__base: ::godot::obj::Base<#base_type>| {
                                    let mut __parent = __base.to_init_gd();
                                    #build_view_values
                                    #name {
                                        base: __base,
                                        #build_fields
                                    }
                                });
                                // <#name as ::moonstone::CustomView>::init(&mut *out.borrow_mut());
                                // <#name as ::moonstone::CustomView>::sync(&mut *out.borrow_mut());
                                out
                            }
                        }
                    }
                }
            }
        }
    }
}
