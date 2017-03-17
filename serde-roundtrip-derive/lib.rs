extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

#[proc_macro_derive(RoundTrip)]
pub fn round_trip(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_macro_input(&s).unwrap();
    let gen = impl_round_trip(&ast);
    gen.parse().unwrap()
}

fn impl_round_trip(ast: &syn::MacroInput) -> quote::Tokens {
    let name = &ast.ident;

    let empty_lifetimes = ast.generics.lifetimes.is_empty();
    let empty_ty_params = ast.generics.ty_params.is_empty();
    let empty_params = empty_lifetimes && empty_ty_params;

    let source_lifetimes_vec = (0..ast.generics.lifetimes.len())
        .map(|n| syn::Ident::from(format!("'a{}", n)))
        .collect::<Vec<_>>();

    let source_ty_params_vec = (0..ast.generics.ty_params.len())
        .map(|n| syn::Ident::from(format!("S{}", n)))
        .collect::<Vec<_>>();

    let source_params_vec = source_lifetimes_vec.iter().cloned()
        .chain(source_ty_params_vec.iter().cloned())
        .collect::<Vec<_>>();

    let target_lifetimes_vec = (0..ast.generics.lifetimes.len())
        .map(|n| syn::Ident::from(format!("'b{}", n)))
        .collect::<Vec<_>>();

    let target_ty_params_vec = (0..ast.generics.ty_params.len())
        .map(|n| syn::Ident::from(format!("T{}", n)))
        .collect::<Vec<_>>();

    let target_params_vec = target_lifetimes_vec.iter().cloned()
        .chain(target_ty_params_vec.iter().cloned())
        .collect::<Vec<_>>();

    let all_params_vec = source_lifetimes_vec.iter().cloned()
        .chain(target_lifetimes_vec.iter().cloned())
        .chain(source_ty_params_vec.iter().cloned())
        .chain(target_ty_params_vec.iter().cloned())
        .chain(::std::iter::once(syn::Ident::from("T")))
        .collect::<Vec<_>>();

    let source_ty_bounds_vec = source_ty_params_vec.iter()
        .zip(target_ty_params_vec.iter())
        .map(|(sty, tty)| quote! { #sty: ::serde_roundtrip::RoundTrip<#tty> })
        .collect::<Vec<_>>();

    let target_ty_bounds_vec = target_ty_params_vec.iter()
        .map(|ty| quote! { #ty: ::serde::Deserialize })
        .collect::<Vec<_>>();

    let source_params = if empty_params {
        quote! { }
    } else {
        quote! { < #(#source_params_vec),* > }
    };

    let target_params = if empty_params {
        quote! { }
    } else {
        quote! { < #(#target_params_vec),* > }
    };

    let all_params = quote! { < #(#all_params_vec),* > };

    let source_ty_bounds = quote! { #(#source_ty_bounds_vec),* };

    let target_ty_bounds = quote! { #(#target_ty_bounds_vec),* };

    let roundtrip_where_clause = if empty_ty_params {
        quote! {
            where T: ::serde_roundtrip::SameDeserialization<SameAs = #name #target_params>,
        }
    } else {
        quote! {
            where T: ::serde_roundtrip::SameDeserialization<SameAs = #name #target_params>,
                #source_ty_bounds, #target_ty_bounds,
        }
    };

    let same_deserialization_where_clause = if empty_ty_params {
        quote! { }
    } else {
        quote! { where #target_ty_bounds }
    };

    let round_trip = match ast.body {
        syn::Body::Struct(syn::VariantData::Struct(ref body)) => {
            let fields = body.iter()
                .filter_map(|field| field.ident.as_ref())
                .map(|ident| quote! { #ident: self.#ident.round_trip() })
                .collect::<Vec<_>>();
            quote! { #name { #(#fields),* } }
        },
        syn::Body::Struct(syn::VariantData::Tuple(ref body)) => {
            let fields = (0..body.len())
                .map(syn::Ident::from)
                .map(|index| quote! { self.#index.round_trip() })
                .collect::<Vec<_>>();
            quote! { #name ( #(#fields),* ) }
        },
        syn::Body::Struct(syn::VariantData::Unit) => {
            quote! { #name }
        },
        syn::Body::Enum(ref body) => {
            let cases = body.iter()
                .map(|case| {
                    let unqualified_ident = &case.ident;
                    let ident = quote! { #name::#unqualified_ident };
                    match case.data {
                        syn::VariantData::Struct(ref body) => {
                            let idents = body.iter()
                                .filter_map(|field| field.ident.as_ref())
                                .collect::<Vec<_>>();;
                            let cloned = idents.iter()
                                .map(|ident| quote! { #ident: #ident.round_trip() })
                                .collect::<Vec<_>>();
                            quote! { #ident { #(ref #idents),* } => #ident { #(#cloned),* } }
                        },
                        syn::VariantData::Tuple(ref body) => {
                            let idents = (0..body.len())
                                .map(|index| syn::Ident::from(format!("x{}", index)))
                                .collect::<Vec<_>>();
                            let cloned = idents.iter()
                                .map(|ident| quote! { #ident.round_trip() })
                                .collect::<Vec<_>>();
                            quote! { #ident ( #(ref #idents),* ) => #ident ( #(#cloned),* ) }
                        },
                        syn::VariantData::Unit => {
                            quote! { #ident => #ident }
                        },
                    }
                })
                .collect::<Vec<_>>();
            quote! { match *self { #(#cases),* } }
        },
    };

    quote! {
        impl #all_params ::serde_roundtrip::RoundTrip<T> for #name #source_params
            // TODO: type bounds in the original type definition
            #roundtrip_where_clause
        {
            fn round_trip(&self) -> T { T::from(#round_trip) }
        }
        impl #target_params ::serde_roundtrip::SameDeserialization for #name #target_params
            // TODO: type bounds in the original type definition
            #same_deserialization_where_clause
        {
            type SameAs = Self;
            fn from(data: Self) -> Self { data }
        }
    }
}
