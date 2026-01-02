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
        base: Box<Type>,
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
    pub impls: TokenStream,
}

fn collect_data(list: &Punctuated<ViewField, Token![,]>, data: &mut DataCollect) {
    for field in list {
        let vis = &field.vis;
        let name = &field.name;
        let priv_name = format_ident!("__DONT_USE_THIS_DIRECTLY_{}", name);

        let typ = &field.typ;
        match (field.view, &field.body) {
            (Some(v), None) => {
                let kw = Ident::new("try", v.span);
                // if *name != "__" {
                data.init_struct_fields.extend(quote! { #vis #name: #typ, });
                data.view_struct_fields
                    .extend(quote! { #vis #priv_name: ::moonstone::ViewValue<#typ>, });

                data.build_view_values.extend(quote! {
                    stringify!(#kw);
                    let __state = <#typ as ::moonstone::View>::build(&self.#name, &mut __parent);
                    let #name = ::moonstone::ViewValue::__create(self.#name, __state);
                });
                data.build_fields.extend(quote! {
                    #priv_name: #name,
                });
                data.impls.extend(quote! {
                    #vis fn #name<'a>(&'a self) -> <#typ as ::moonstone::View>::Access<'a> {
                        <#typ as ::moonstone::View>::access(self.#priv_name.__value())
                    }
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
            (kw, Some(body)) => {
                let Some(kw) = kw else {
                    panic!("Bruuuhhh put view there");
                };
                let kw = Ident::new("try", kw.span);

                data.init_struct_fields
                    .extend(quote! { #vis #name: ::godot::obj::Gd<#typ>, });
                data.view_struct_fields
                    .extend(quote! { #vis #priv_name: ::godot::obj::Gd<#typ>, });

                data.build_view_values.extend(quote! {
                    stringify!(#kw);
                    __parent.node().add_child(&self.#name);
                    let mut __parent = ::moonstone::ChildAnchor::new(self.#name.clone().upcast());
                });
                // if *name != "__" {
                data.build_fields.extend(quote! {
                    #priv_name: self.#name,
                });
                // }

                collect_data(body, data);

                data.build_view_values.extend(quote! {
                    let mut __parent = ::moonstone::ChildAnchor::new(__parent.node().get_parent().unwrap());
                });
                data.impls.extend(quote! {
                    #vis fn #name(&self) -> ::godot::obj::Gd<#typ> {
                        self.#priv_name.clone()
                    }
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
                    impls: quote! {},
                };

                collect_data(body, &mut collect);

                let mod_name = format_ident!("_mod_{}", name);
                let init_struct_name = format_ident!("{}_Init", name);
                let DataCollect {
                    init_struct_fields,
                    view_struct_fields,
                    build_view_values,
                    build_fields,
                    impls,
                } = collect;

                quote! {

                    #[derive(::godot::prelude::GodotClass)]
                    #[class(base=#base, no_init)]
                    #[allow(non_snake_case)]
                    #vis struct #name {
                        base: ::godot::obj::Base<#base>,
                        #view_struct_fields
                    }
                    #[allow(non_camel_case_types)]
                    #vis struct #init_struct_name {
                        #init_struct_fields
                    }

                    impl #init_struct_name {
                        pub fn build(self, f: impl FnOnce(&mut ::godot::obj::Gd<#name>)) -> ::godot::obj::Gd<#name> {
                            use ::moonstone::Anchor;
                            use ::godot::obj::NewAlloc;
                            let mut out = ::godot::obj::Gd::from_init_fn(|__base: ::godot::obj::Base<#base>| {
                                let mut __node = __base.to_init_gd();
                                let mut __parent = ::moonstone::ChildAnchor::new(__node.upcast());
                                #build_view_values
                                #name {
                                    base: __base,
                                    #build_fields
                                }
                            });
                            // f(&mut *out.bind_mut());
                            // <#name as ::moonstone::CustomView>::init(&mut *out.bind_mut());

                            out
                        }
                    }
                    impl #name {
                        #impls
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
                        .extend(quote! { #variant(<#typ as ::moonstone::View>::State), });
                    build_match.extend(quote! {
                        #name::#variant(v) => #view_state_name::#variant(v.build(enum_anchor)),
                    });
                    rebuild_match.extend(quote! {
                        (#name::#variant(new), #view_state_name::#variant(inner_state)) => {
                            new.rebuild(inner_state);
                            return;
                        },
                    });
                    teardown_match.extend(quote! {
                        #view_state_name::#variant(inner_state) => {
                            <#typ as ::moonstone::View>::teardown(inner_state, enum_anchor);
                        },
                    });
                    collect_match.extend(quote! {
                        #view_state_name::#variant(inner_state) => {
                            <#typ as ::moonstone::View>::collect_nodes(inner_state, nodes);
                        },
                    });
                }
                quote! {
                    #vis enum #name {
                        #variant_gen
                    }
                    #[allow(non_camel_case_types)]
                    #vis enum #view_state_name {
                        #view_state_variant_gen
                    }
                    impl ::moonstone::View for #name {
                        type State = (::moonstone::BeforeAnchor, #view_state_name);
                        type Access<'a> = &'a Self where Self: 'a;

                        fn build(&self, parent_anchor: &mut dyn ::moonstone::Anchor) -> Self::State {
                            use ::moonstone::Anchor;
                            let mut enum_anchor_owned = <::moonstone::BeforeAnchor as ::moonstone::Anchor>::new(<::godot::classes::Node as ::godot::obj::NewAlloc>::new_alloc());
                            let enum_anchor = &mut enum_anchor_owned;
                            parent_anchor.add(&enum_anchor.node());

                            let inner_state = match self {
                                #build_match
                            };

                            (
                                enum_anchor_owned,
                                inner_state,
                            )
                        }

                        fn rebuild(&self, state: &mut Self::State) {
                            use ::moonstone::Anchor;
                            let enum_anchor = &mut state.0;
                            match (self, &mut state.1) {
                                #rebuild_match
                                _ => {}
                            }
                            match &mut state.1 {
                                #teardown_match
                            }
                            let inner_state = match self {
                                #build_match
                            };
                            state.1 = inner_state;
                        }

                        fn teardown(state: &mut Self::State, parent_anchor: &mut dyn ::moonstone::Anchor) {
                            use ::moonstone::Anchor;
                            let enum_anchor = &mut state.0;
                            match &mut state.1 {
                                #teardown_match
                            }
                            parent_anchor.remove(&state.0.node());
                            state.0.node().queue_free();
                        }

                        fn collect_nodes(state: &Self::State, nodes: &mut Vec<::godot::obj::Gd<::godot::classes::Node>>) {
                            use ::moonstone::Anchor;
                            nodes.push(state.0.node());
                            match &state.1 {
                                #collect_match
                            }
                        }

                        fn access<'a>(&'a self) -> Self::Access<'a> {
                            self
                        }
                    }
                }
            }
        }
    }
}
