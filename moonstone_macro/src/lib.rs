mod viewtype;

use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Field, Fields, ItemStruct, Lit, LitInt, Meta, MetaNameValue,
    Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

use crate::viewtype::ViewDef;

fn parse_args(input: ParseStream) -> syn::Result<Punctuated<MetaNameValue, Token![,]>> {
    Punctuated::parse_terminated(input)
}

/// viewtype! { ... }
#[proc_macro]
pub fn viewtype(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let b = parse_macro_input!(item as ViewDef);

    b.gen_rust().into()
}

// #[proc_macro_attribute]
// pub fn view(args: TokenStream, input: TokenStream) -> TokenStream {
//     let args = parse_macro_input!(args with parse_args);

//     let base_type = &args
//         .iter()
//         .find(|v| v.path.is_ident("base"))
//         .expect("Need `base` value")
//         .value;
//     // let msg_type = &args
//     //     .iter()
//     //     .find(|v| v.path.is_ident("msg"))
//     //     .expect("Need `msg` value")
//     //     .value;

//     let input = parse_macro_input!(input as ItemStruct);
//     let struct_name = &input.ident;
//     let struct_vis = &input.vis;

//     let Fields::Named(fields_named) = input.fields else {
//         panic!("Only structs with named fields can be views")
//     };

//     let mut init_struct_fields = quote! {};
//     let mut view_struct_fields = quote! {};

//     let mut build_view_values = quote! {};
//     let mut build_fields = quote! {};

//     // let mut fwd_rebuild = quote! {};
//     // let mut fwd_msg = quote! {};

//     for field in &fields_named.named {
//         let name = &field.ident.clone().unwrap();
//         let typ = &field.ty;
//         let vis = &field.vis;
//         let is_view = field.attrs.iter().any(|v| v.path().is_ident("view"));
//         let enter = field
//             .attrs
//             .iter()
//             .find(|v| v.path().is_ident("enter"))
//             .map(|v| {
//                 v.parse_args_with(|i: ParseStream| {
//                     Punctuated::<Field, Token![,]>::parse_terminated_with(i, Field::parse_named)
//                 })
//                 .expect("`enter` argument must be a list of fields")
//             });
//         let exit = field
//             .attrs
//             .iter()
//             .find(|v| v.path().is_ident("exit"))
//             .map(|v| {
//                 v.parse_args_with(|i: ParseStream| LitInt::parse(i))
//                     .expect("`exit` argument must be an integer")
//             });

//         if is_view {
//             if let Some(exit) = exit {
//                 for _ in 0..exit.base10_parse::<u32>().unwrap() {
//                     build_view_values.extend(quote! {
//                         let mut __parent = __parent.get_parent().unwrap();
//                     });
//                 }
//             }
//             if let Some(enter) = enter {
//                 for field in enter {
//                     let vis = &field.vis;
//                     let name = &field.ident.unwrap();
//                     let typ = &field.ty;
//                     init_struct_fields.extend(quote! { #vis #name: ::godot::obj::Gd<#typ>, });
//                     view_struct_fields.extend(quote! { #vis #name: ::godot::obj::Gd<#typ>, });
//                     build_view_values.extend(quote! {
//                         __parent.add_child(&self.#name);
//                         let mut __parent = self.#name.clone();
//                     });
//                     build_fields.extend(quote! {
//                         #name: self.#name,
//                     });
//                 }
//             }
//             init_struct_fields.extend(quote! { #vis #name: #typ, });
//             view_struct_fields.extend(quote! { #vis #name: ::moonstone::ViewValue<#typ>, });

//             build_view_values.extend(quote! {
//                 let __state = <#typ as ::moonstone::View>::build(&self.#name, __parent.clone().upcast(), ::moonstone::AnchorType::ChildOf);
//                 let #name = ::moonstone::ViewValue::create(self.#name, __state);
//             });
//             build_fields.extend(quote! {
//                 #name,
//             });
//             // fwd_msg.extend(quote! {
//             //     <#typ as ::moonstone::View>::message(&mut self.#name.__value, msg, &mut self.#name.__state);
//             // });
//         } else {
//             init_struct_fields.extend(quote! { #vis #name: #typ, });
//             view_struct_fields.extend(quote! { #vis #name: #typ, });
//             build_fields.extend(quote! {
//                 #name: self.#name,
//             });
//         }
//     }

//     let mod_name = format_ident!("_def_{}", struct_name);
//     let init_struct_name = format_ident!("{}_Init", struct_name);
//     let view_struct_name = format_ident!("{}_View", struct_name);
//     let weak_struct_name = format_ident!("{}_Weak", struct_name);

//     let expanded = quote! {

//         #[doc(hidden)]
//         #[allow(non_snake_case)]
//         mod #mod_name {
//             use super::*;
//             use std::rc::{Rc, Weak};
//             use std::cell::{RefCell, Ref, RefMut};

//             #[allow(non_camel_case_types)]
//             pub struct #view_struct_name {
//                 inner: Rc<RefCell<#struct_name>>,
//             }
//             #[allow(non_camel_case_types)]
//             #[derive(Clone)]
//             pub struct #weak_struct_name {
//                 inner: Weak<RefCell<#struct_name>>,
//             }
//             pub struct #struct_name {
//                 base: ::godot::obj::Gd<#base_type>,
//                 weak: #weak_struct_name,
//                 #view_struct_fields
//             }
//             #[allow(non_camel_case_types)]
//             pub struct #init_struct_name {
//                 #init_struct_fields
//             }
//             impl #view_struct_name {
//                 fn borrow(&self) -> Ref<#struct_name> {
//                     self.inner.borrow()
//                 }
//                 fn borrow_mut(&self) -> RefMut<#struct_name> {
//                     self.inner.borrow_mut()
//                 }
//                 pub fn with<R>(&self, f: impl FnOnce(&#struct_name) -> R) -> R {
//                     f(&*self.inner.borrow())
//                 }
//                 pub fn update<R>(&self, f: impl FnOnce(&mut #struct_name) -> R) -> R {
//                     let out = f(&mut *self.inner.borrow_mut());
//                     <#struct_name as ::moonstone::CustomView>::sync(&mut *self.borrow_mut());
//                     out
//                 }
//             }
//             impl #weak_struct_name {
//                 pub fn with<R>(&self, f: impl FnOnce(&#struct_name) -> R) -> Option<R> {
//                     self.inner.upgrade().map(|v| f(&*v.borrow()))
//                 }
//                 pub fn update<R>(&self, f: impl FnOnce(&mut #struct_name) -> R) -> Option<R> {
//                     self.inner.upgrade().map(|v| {
//                         let out = f(&mut *v.borrow_mut());
//                         <#struct_name as ::moonstone::CustomView>::sync(&mut *v.borrow_mut());
//                         out
//                     })
//                 }
//             }
//             impl #struct_name {
//                 pub fn weak(&self) -> #weak_struct_name {
//                     self.weak.clone()
//                 }
//             }
//             impl #init_struct_name {
//                 pub fn build(self) -> #view_struct_name {
//                     use ::godot::obj::NewAlloc;
//                     let base = #base_type::new_alloc();
//                     let mut __parent = base.clone();
//                     #build_view_values
//                     let inner = Rc::new(RefCell::new(#struct_name {
//                         base,
//                         weak: #weak_struct_name {
//                             inner: Weak::new(),
//                         },
//                         #build_fields
//                     }));
//                     let weak = Rc::downgrade(&inner);
//                     inner.borrow_mut().weak.inner = weak;
//                     let mut out = #view_struct_name {
//                         inner,
//                     };
//                     <#struct_name as ::moonstone::CustomView>::init(&mut *out.borrow_mut());
//                     <#struct_name as ::moonstone::CustomView>::sync(&mut *out.borrow_mut());
//                     out
//                 }
//             }
//             impl ::moonstone::View for #view_struct_name {
//                 type State = ::godot::obj::Gd<#base_type>;

//                 fn build(
//                     &self,
//                     mut parent_anchor: ::godot::obj::Gd<godot::prelude::Node>,
//                     parent_anchor_type: ::moonstone::AnchorType,
//                 ) -> ::moonstone::ViewState<Self> {
//                     let base = self.borrow().base.clone();
//                     parent_anchor_type.add(&mut parent_anchor, &base.clone().upcast());
//                     ::moonstone::ViewState {
//                         state: base,
//                         parent_anchor: parent_anchor,
//                         parent_anchor_type: parent_anchor_type,
//                     }
//                 }

//                 fn rebuild(&self, state: &mut ::moonstone::ViewState<Self>) {
//                     let base = self.borrow().base.clone();
//                     if base.upcast_ref::<::godot::classes::Node>().get_parent() != state.state.upcast_ref::<::godot::classes::Node>().get_parent() {
//                         state.state.upcast_mut::<::godot::classes::Node>().queue_free();
//                         state.state.clone().upcast_mut::<::godot::classes::Node>().replace_by(&base);
//                         state.state = base.clone();
//                     }
//                 }

//                 fn teardown(state: &mut ::moonstone::ViewState<Self>) {
//                     let mut node = state.state.clone().upcast::<::godot::classes::Node>();
//                     state
//                         .parent_anchor_type
//                         .remove(&mut state.parent_anchor, &node);
//                     node.queue_free();
//                 }

//                 fn collect_nodes(state: &::moonstone::ViewState<Self>, nodes: &mut Vec<::godot::obj::Gd<::godot::classes::Node>>) {
//                     nodes.push(state.state.clone().upcast());
//                 }
//             }
//         }
//         #struct_vis use #mod_name::{#struct_name, #view_struct_name, #weak_struct_name, #init_struct_name};
//     };

//     TokenStream::from(expanded)
// }
