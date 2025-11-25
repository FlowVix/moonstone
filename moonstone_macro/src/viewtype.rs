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
                let view_struct_name = format_ident!("{}_View", name);
                let weak_struct_name = format_ident!("{}_Weak", name);
                let DataCollect {
                    init_struct_fields,
                    view_struct_fields,
                    build_view_values,
                    build_fields,
                } = collect;

                quote! {

                    #[allow(non_camel_case_types)]
                    #vis struct #view_struct_name {
                        #[doc(hidden)]
                        __inner: std::rc::Rc<std::cell::RefCell<#name>>,
                    }
                    #[allow(non_camel_case_types)]
                    #[derive(Clone)]
                    #vis struct #weak_struct_name {
                        #[doc(hidden)]
                        __inner: std::rc::Weak<std::cell::RefCell<#name>>,
                    }
                    #vis struct #name {
                        #[doc(hidden)]
                        __base: ::godot::obj::Gd<#base_type>,
                        #[doc(hidden)]
                        __weak: #weak_struct_name,
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


                        impl #view_struct_name {
                            fn borrow(&self) -> Ref<#name> {
                                self.__inner.borrow()
                            }
                            fn borrow_mut(&self) -> RefMut<#name> {
                                self.__inner.borrow_mut()
                            }
                            pub fn with<R>(&self, f: impl FnOnce(&#name) -> R) -> R {
                                f(&*self.__inner.borrow())
                            }
                            pub fn update<R>(&self, f: impl FnOnce(&mut #name) -> R) -> R {
                                let out = f(&mut *self.__inner.borrow_mut());
                                <#name as ::moonstone::CustomView>::sync(&mut *self.borrow_mut());
                                out
                            }
                        }
                        impl #weak_struct_name {
                            pub fn with<R>(&self, f: impl FnOnce(&#name) -> R) -> Option<R> {
                                self.__inner.upgrade().map(|v| f(&*v.borrow()))
                            }
                            pub fn update<R>(&self, f: impl FnOnce(&mut #name) -> R) -> Option<R> {
                                self.__inner.upgrade().map(|v| {
                                    let out = f(&mut *v.borrow_mut());
                                    <#name as ::moonstone::CustomView>::sync(&mut *v.borrow_mut());
                                    out
                                })
                            }
                        }
                        impl #name {
                            pub fn weak(&self) -> #weak_struct_name {
                                self.__weak.clone()
                            }
                        }
                        impl #init_struct_name {
                            pub fn build(self) -> #view_struct_name {
                                use ::godot::obj::NewAlloc;
                                let __base = #base_type::new_alloc();
                                let mut __parent = __base.clone();
                                #build_view_values
                                let __inner = Rc::new(RefCell::new(#name {
                                    __base,
                                    __weak: #weak_struct_name {
                                        __inner: Weak::new(),
                                    },
                                    #build_fields
                                }));
                                let __weak = Rc::downgrade(&__inner);
                                __inner.borrow_mut().__weak.__inner = __weak;
                                let mut out = #view_struct_name {
                                    __inner,
                                };
                                <#name as ::moonstone::CustomView>::init(&mut *out.borrow_mut());
                                <#name as ::moonstone::CustomView>::sync(&mut *out.borrow_mut());
                                out
                            }
                        }
                        impl ::moonstone::View for #view_struct_name {
                            type State = ::godot::obj::Gd<#base_type>;

                            fn build(
                                &self,
                                mut parent_anchor: ::godot::obj::Gd<godot::prelude::Node>,
                                parent_anchor_type: ::moonstone::AnchorType,
                            ) -> ::moonstone::ViewState<Self> {
                                let base = self.borrow().__base.clone();
                                parent_anchor_type.add(&mut parent_anchor, &base.clone().upcast());
                                ::moonstone::ViewState {
                                    state: base,
                                    parent_anchor: parent_anchor,
                                    parent_anchor_type: parent_anchor_type,
                                }
                            }

                            fn rebuild(&self, state: &mut ::moonstone::ViewState<Self>) {
                                let base = self.borrow().__base.clone();
                                if base.upcast_ref::<::godot::classes::Node>().get_parent() != state.state.upcast_ref::<::godot::classes::Node>().get_parent() {
                                    state.state.upcast_mut::<::godot::classes::Node>().queue_free();
                                    state.state.clone().upcast_mut::<::godot::classes::Node>().replace_by(&base);
                                    state.state = base.clone();
                                }
                            }

                            fn teardown(state: &mut ::moonstone::ViewState<Self>) {
                                let mut node = state.state.clone().upcast::<::godot::classes::Node>();
                                state
                                    .parent_anchor_type
                                    .remove(&mut state.parent_anchor, &node);
                                node.queue_free();
                            }

                            fn collect_nodes(state: &::moonstone::ViewState<Self>, nodes: &mut Vec<::godot::obj::Gd<::godot::classes::Node>>) {
                                nodes.push(state.state.clone().upcast());
                            }
                        }
                    }
                }
            }
        }
    }
}
