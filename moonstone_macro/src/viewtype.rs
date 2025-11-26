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
    typ: ViewType,
}
pub enum ViewType {
    Struct {
        name: Ident,
        base: Type,
        body: Punctuated<ViewField, Token![,]>,
    },
    Enum {
        name: Ident,
        variants: Punctuated<ViewVariant, Token![,]>,
    },
}

pub struct ViewVariant {
    name: Ident,
    typ: Type,
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
                typ: ViewType::Struct { name, base, body },
            })
        } else if input.peek(Token![enum]) {
            input.parse::<Token![enum]>()?;
            let name = input.parse()?;
            let inner;
            braced!(inner in input);
            let variants = Punctuated::parse_terminated(&inner)?;
            Ok(ViewDef {
                vis,
                typ: ViewType::Enum { name, variants },
            })
        } else {
            panic!("Struct or enum bro")
        }
    }
}

impl Parse for ViewVariant {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let inner;
        parenthesized!(inner in input);
        let typ = inner.parse()?;
        Ok(ViewVariant { name, typ })
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
        let vis = &self.vis;
        match &self.typ {
            ViewType::Struct { name, base, body } => {
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
                    #[class(base=#base, no_init)]
                    #vis struct #name {
                        base: ::godot::obj::Base<#base>,
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
                                let out = ::godot::obj::Gd::from_init_fn(|__base: ::godot::obj::Base<#base>| {
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
            ViewType::Enum { name, variants } => {
                let view_state_name = format_ident!("__{}_ViewStateType", name);

                let mut variant_gen = quote! {};
                let mut view_state_variant_gen = quote! {};
                let mut build_match = quote! {};
                let mut rebuild_match = quote! {};
                let mut teardown_match = quote! {};
                let mut collect_match = quote! {};
                for i in variants {
                    let variant = &i.name;
                    let typ = &i.typ;
                    variant_gen.extend(quote! { #variant(#typ), });
                    view_state_variant_gen
                        .extend(quote! { #variant(::moonstone::ViewState<#typ>), });
                    build_match.extend(quote! {
                        #name::#variant(v) => #view_state_name::#variant(v.build(enum_anchor.clone(), ::moonstone::AnchorType::Before)),
                    });
                    rebuild_match.extend(quote! {
                        (#name::#variant(new), #view_state_name::#variant(inner_state)) => {
                            new.rebuild(inner_state);
                            return;
                        },
                    });
                    teardown_match.extend(quote! {
                        #view_state_name::#variant(inner_state) => {
                            ::moonstone::View::teardown(inner_state);
                        },
                    });
                    collect_match.extend(quote! {
                        #view_state_name::#variant(inner_state) => {
                            ::moonstone::View::collect_nodes(inner_state, nodes);
                        },
                    });
                }
                quote! {
                    #vis enum #name {
                        #variant_gen
                    }
                    #[allow(non_camel_case_types)]
                    enum #view_state_name {
                        #view_state_variant_gen
                    }
                    impl ::moonstone::View for #name {
                        type State = (::godot::obj::Gd<::godot::classes::Node>, #view_state_name);

                        fn build(
                            &self,
                            mut parent_anchor: ::godot::obj::Gd<::godot::classes::Node>,
                            parent_anchor_type: ::moonstone::AnchorType,
                        ) -> ::moonstone::ViewState<Self> {
                            let enum_anchor = <::godot::classes::Node as ::godot::obj::NewAlloc>::new_alloc();
                            parent_anchor_type.add(&mut parent_anchor, &enum_anchor);

                            let inner_state = match self {
                                #build_match
                            };

                            ::moonstone::ViewState {
                                state: (
                                    enum_anchor,
                                    inner_state,
                                ),
                                parent_anchor,
                                parent_anchor_type,
                            }
                        }

                        fn rebuild(&self, state: &mut ::moonstone::ViewState<Self>) {
                            let enum_anchor = state.state.0.clone();
                            match (self, &mut state.state.1) {
                                #rebuild_match
                                _ => {}
                            }
                            match &mut state.state.1 {
                                #teardown_match
                            }
                            let inner_state = match self {
                                #build_match
                            };
                            state.state.1 = inner_state;
                        }

                        fn teardown(state: &mut ::moonstone::ViewState<Self>) {
                            match &mut state.state.1 {
                                #teardown_match
                            }
                            state
                                .parent_anchor_type
                                .remove(&mut state.parent_anchor, &state.state.0);
                            state.state.0.queue_free();
                        }

                        fn collect_nodes(state: &::moonstone::ViewState<Self>, nodes: &mut Vec<::godot::obj::Gd<::godot::classes::Node>>) {
                            nodes.push(state.state.0.clone());
                            match &state.state.1 {
                                #collect_match
                            }
                        }
                    }
                }
            }
        }
    }
}
