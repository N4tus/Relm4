use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{parse_quote, Visibility};

use crate::visitors::{ComponentVisitor, PreAndPostView};

pub(super) mod inject_view_code;
pub(crate) mod token_streams;

use inject_view_code::inject_view_code;

pub(crate) fn generate_tokens(
    vis: Option<Visibility>,
    mut component_impl: syn::ItemImpl,
) -> TokenStream2 {
    let mut errors = vec![];

    let mut component_visitor = ComponentVisitor::new(&mut errors);
    component_visitor.visit_item_impl_mut(&mut component_impl);

    let additional_fields = component_visitor.additional_fields.take();

    let menus_stream = component_visitor
        .menus
        .take()
        .map(|menus| menus.menus_stream());

    let mut struct_fields = None;

    if let ComponentVisitor {
        view_widgets: Some(view_widgets),
        model_name: Some(model_name),
        root_name: Some(root_name),
        init: Some(init),
        errors,
        ..
    } = component_visitor
    {
        let token_streams::TokenStreams {
            error,
            init_root,
            rename_root,
            struct_fields: struct_fields_stream,
            init: init_widgets,
            assign,
            connect,
            return_fields,
            destructure_fields,
            update_view,
        } = view_widgets.generate_streams(&vis, &model_name, Some(&root_name), false);

        struct_fields = Some(struct_fields_stream);
        let root_widget_type = view_widgets.root_type();

        // Extract identifiers from additional fields for struct initialization: "test: u8" => "test"
        let additional_fields_return_stream = if let Some(fields) = &additional_fields {
            let mut tokens = TokenStream2::new();
            for field in fields.inner.pairs() {
                tokens.extend(field.value().ident.to_token_stream());
                tokens.extend(quote! {,});
            }
            tokens
        } else {
            TokenStream2::new()
        };

        let view_code = quote! {
            #rename_root
            #menus_stream
            #init_widgets
            #connect
            {
                #error
            }
            #assign
        };

        let widgets_return_code = quote! {
            Self::Widgets {
                #return_fields
                #additional_fields_return_stream
            }
        };

        let init_injected = match inject_view_code(init, view_code, widgets_return_code) {
            Ok(method) => method,
            Err(err) => return err.to_compile_error(),
        };

        component_impl.items.push(parse_quote! {
            type Root = #root_widget_type;
        });

        component_impl.items.push(parse_quote! {
            fn init_root() -> Self::Root {
                #init_root
            }
        });

        let PreAndPostView {
            pre_view,
            post_view,
            ..
        } = PreAndPostView::extract(&mut component_impl, errors);

        component_impl.items.push(parse_quote! {
            /// Update the view to represent the updated model.
            fn update_view(
                &self,
                widgets: &mut Self::Widgets,
                sender: ComponentSender<Self>,
            ) {
                struct __DoNotReturnManually;

                let _no_manual_return: __DoNotReturnManually = (move || {
                    #[allow(unused_variables)]
                    let Self::Widgets {
                        #destructure_fields
                        #additional_fields_return_stream
                    } = widgets;

                    #[allow(unused_variables)]
                    let #model_name = self;

                    #(#pre_view)*
                    #update_view
                    // In post_view returning early is ok
                    (move || { #(#post_view)* })();

                    __DoNotReturnManually
                })();
            }
        });

        component_impl
            .items
            .push(syn::ImplItem::Method(init_injected));
    }

    let outer_attrs = &component_impl.attrs;
    let widgets_struct = component_visitor.widgets_ty.map(|ty| {
        quote! {
            #[allow(dead_code)]
            #(#outer_attrs)*
            #[derive(Debug)]
            #vis struct #ty {
                #struct_fields
                #additional_fields
            }
        }
    });

    let errors = errors.iter().map(syn::Error::to_compile_error);

    quote! {
        #widgets_struct

        #component_impl

        #(#errors)*
    }
}
