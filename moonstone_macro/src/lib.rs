use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Fields, ItemStruct, Lit, Meta, MetaNameValue, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

fn parse_args(input: ParseStream) -> syn::Result<Punctuated<MetaNameValue, Token![,]>> {
    Punctuated::parse_terminated(input)
}

#[proc_macro_attribute]
pub fn view(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with parse_args);

    let base_type = &args
        .iter()
        .find(|v| v.path.is_ident("base"))
        .expect("Need `base` value")
        .value;
    let msg_type = &args
        .iter()
        .find(|v| v.path.is_ident("msg"))
        .expect("Need `msg` value")
        .value;
    let cb_expr = &args
        .iter()
        .find(|v| v.path.is_ident("cb"))
        .expect("Need `cb` value")
        .value;

    let input = parse_macro_input!(input as ItemStruct);
    let struct_name = &input.ident;
    let struct_vis = &input.vis;

    let Fields::Named(fields_named) = input.fields else {
        panic!("Only structs with named fields can be views")
    };

    // let mut field_names = Vec::new();
    let mut init_struct_fields = quote! {};
    let mut view_struct_fields = quote! {};

    // let mut accessors = quote! {};

    let mut build_view_values = quote! {};
    let mut build_fields = quote! {};

    let mut fwd_msg = quote! {};

    for field in &fields_named.named {
        let name = &field.ident.clone().unwrap();
        let typ = &field.ty;
        let vis = &field.vis;
        let is_view = field.attrs.iter().any(|v| v.path().is_ident("view"));

        init_struct_fields.extend(quote! { #vis #name: #typ, });

        if is_view {
            view_struct_fields
                .extend(quote! { #vis #name: ::moonstone::ViewValue<#msg_type, #typ>, });

            // let mut_struct_name = format_ident!(
            //     "__{}{}Mut",
            //     struct_name,
            //     name.to_string().to_upper_camel_case()
            // );
            // let mut_acc_name = format_ident!("{}_mut", name,);
            // mut_structs.extend(quote! {
            //     pub struct #mut_struct_name<'a> {
            //         base: ::godot::obj::Gd<#base_type>,
            //         v: &'a mut ::moonstone::ViewValue<#msg_type, #typ>,
            //     }
            //     impl<'a> std::ops::Deref for #mut_struct_name<'a> {
            //         type Target = #typ;

            //         fn deref(&self) -> &Self::Target {
            //             &self.v.__value
            //         }
            //     }
            //     impl<'a> std::ops::DerefMut for #mut_struct_name<'a> {
            //         fn deref_mut(&mut self) -> &mut Self::Target {
            //             &mut self.v.__value
            //         }
            //     }
            //     impl<'a> std::ops::Drop for #mut_struct_name<'a> {
            //         fn drop(&mut self) {
            //             use ::moonstone::View;
            //             self.v.__value.rebuild(&mut self.v.__state)
            //         }
            //     }
            // });
            // accessors.extend(quote! {
            //     #vis fn #name(&self) -> &#typ {
            //         &self.#name.__value
            //     }
            //     #vis fn #mut_acc_name(&mut self) -> &mut  {
            //         #mut_struct_name {
            //             base: self.base.clone(),
            //             v: &mut self.#name,
            //         }
            //     }
            // });
            build_view_values.extend(quote! {
                let state = <#typ as ::moonstone::View<#msg_type>>::build(&self.#name, base.clone().upcast(), ::moonstone::AnchorType::ChildOf);
                let #name = ::moonstone::ViewValue {
                    __state: state,
                    __value: self.#name,
                };
            });
            build_fields.extend(quote! {
                #name,
            });
            fwd_msg.extend(quote! {
                self.#name.message(msg);
            });
        } else {
            view_struct_fields.extend(quote! { #vis #name: #typ, });
            build_fields.extend(quote! {
                #name: self.#name,
            });
        }
    }

    let mod_name = format_ident!("_def_{}", struct_name);
    let init_struct_name = format_ident!("{}Init", struct_name);
    let cb_name = format_ident!("__CB__{}", struct_name.to_string().to_uppercase());

    let expanded = quote! {
        #[doc(hidden)]
        const #cb_name: fn(&mut #struct_name, &#msg_type) = #cb_expr;

        #[doc(hidden)]
        #[allow(non_snake_case)]
        mod #mod_name {
            use super::*;

            pub struct #struct_name {
                base: ::godot::obj::Gd<#base_type>,
                #view_struct_fields
            }
            pub struct #init_struct_name {
                #init_struct_fields
            }
            // #mut_structs
            // impl #struct_name {
            //     #accessors
            // }
            impl #init_struct_name {
                pub fn build(self) -> #struct_name {
                    use ::godot::obj::NewAlloc;
                    let base = #base_type::new_alloc();
                    #build_view_values
                    #struct_name {
                        base,
                        #build_fields
                    }
                }
            }
            impl ::moonstone::View<#msg_type> for #struct_name {
                type State = ::godot::obj::Gd<#base_type>;

                fn build(
                    &self,
                    mut parent_anchor: ::godot::obj::Gd<godot::prelude::Node>,
                    parent_anchor_type: ::moonstone::AnchorType,
                ) -> ::moonstone::ViewState<#msg_type, Self> {
                    parent_anchor_type.add(&mut parent_anchor, &self.base.clone().upcast());
                    ::moonstone::ViewState {
                        __state: self.base.clone(),
                        __parent_anchor: parent_anchor,
                        __parent_anchor_type: parent_anchor_type,
                    }
                }

                fn rebuild(&self, state: &mut ::moonstone::ViewState<#msg_type, Self>) {
                    ::moonstone::View::<#msg_type>::teardown(state);
                    state
                        .__parent_anchor_type
                        .add(&mut state.__parent_anchor, &self.base.clone().upcast());
                    state.__state = self.base.clone();
                }

                fn teardown(state: &mut ::moonstone::ViewState<#msg_type, Self>) {
                    let node = state.__state.clone().upcast();
                    state
                        .__parent_anchor_type
                        .remove(&mut state.__parent_anchor, &node);
                }

                fn collect_nodes(state: &::moonstone::ViewState<#msg_type, Self>, nodes: &mut Vec<::godot::obj::Gd<::godot::classes::Node>>) {
                    nodes.push(state.__state.clone().upcast());
                }

                fn message(&mut self, msg: &#msg_type) {
                    #cb_name(self, msg);
                    #fwd_msg
                }
            }
        }
        #struct_vis use #mod_name::{#struct_name, #init_struct_name};
    };

    TokenStream::from(expanded)
}
