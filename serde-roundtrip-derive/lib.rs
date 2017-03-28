extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::fold::Folder;
use syn::fold::noop_fold_generics;
use syn::fold::noop_fold_path;
use syn::AngleBracketedParameterData;
use syn::Generics;
use syn::Ident;
use syn::Lifetime;
use syn::Path;
use syn::PathParameters;
use syn::PathSegment;
use syn::PolyTraitRef;
use syn::TraitBoundModifier;
use syn::Ty;
use syn::TyParam;
use syn::TyParamBound;
use syn::WhereClause;

#[proc_macro_derive(RoundTrip)]
pub fn round_trip(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_macro_input(&s).unwrap();
    let gen = impl_round_trip(&ast);
    gen.parse().unwrap()
}

// Rename the generics in a generic type declaration.

struct Renaming<'a> {
    original: &'a Generics,
    lifetime_prefix: &'a str,
    ty_param_prefix: &'a str,
}

impl<'a> Folder for Renaming<'a> {
    fn fold_generics(&mut self, generics: Generics) -> Generics {
        let mut result = noop_fold_generics(self, generics);
        for ty_param in &mut result.ty_params {
            ty_param.ident = self.fold_ty_param_ident(ty_param.ident.clone());
        }
        result
    }
    fn fold_lifetime(&mut self, lifetime: Lifetime) -> Lifetime {
        Lifetime { ident: self.fold_lifetime_ident(lifetime.ident) }
    }
    fn fold_path(&mut self, path: Path) -> Path {
        let mut result = noop_fold_path(self, path);
        if let Some((segment, rest)) = result.segments.split_first_mut() {
            if rest.is_empty() && segment.parameters.is_empty() {
                segment.ident = self.fold_ty_param_ident(segment.ident.clone());
            }
        }
        result
    }
}

impl<'a> Renaming<'a> {
    fn fold_lifetime_ident(&mut self, ident: Ident) -> Ident {
        self.original.lifetimes.iter()
            .position(|original| original.lifetime.ident == ident)
            .map(|index| syn::Ident::from(format!("{}{}", self.lifetime_prefix, index)))
            .unwrap_or(ident)
    }
    fn fold_ty_param_ident(&mut self, ident: Ident) -> Ident {
        self.original.ty_params.iter()
            .position(|original| original.ident == ident)
            .map(|index| syn::Ident::from(format!("{}{}", self.ty_param_prefix, index)))
            .unwrap_or(ident)
    }
}

// Convert an ident with its generic parameters to a path

fn generic_path(ident: &Ident, generics: &Generics) -> Path {
    Path {
        global: false,
        segments: vec![ PathSegment {
            ident: ident.clone(),
            parameters: PathParameters::AngleBracketed(AngleBracketedParameterData {
                lifetimes: generics.lifetimes.iter()
                    .map(|lifetime_def| lifetime_def.lifetime.clone())
                    .collect(),
                types: generics.ty_params.iter()
                    .map(|ty_param| Ty::Path(None, Path::from(ty_param.ident.clone())))
                    .collect(),
                bindings: vec![],
            }),
        } ]
    }
}

// A type bound

fn ty_param_bound(text: &str) -> TyParamBound {
    TyParamBound::Trait(
        PolyTraitRef {
            bound_lifetimes: vec![],
            trait_ref: syn::parse::path(text).expect("Unexpected parse error"),
        },
        TraitBoundModifier::None,
    )
}

// Derive a RoundTrip implementation

fn impl_round_trip(ast: &syn::MacroInput) -> quote::Tokens {
    let name = &ast.ident;

    // If the original is Foo<'l, X, Y>, the target type is Foo<'b0, T0, T1>.
    let mut target_renaming = Renaming { original: &ast.generics, lifetime_prefix: "'b", ty_param_prefix: "T" };
    let mut target_generics = target_renaming.fold_generics(ast.generics.clone());
    for ty_param in target_generics.ty_params.iter_mut() {
        ty_param.bounds.push(ty_param_bound("::serde::Deserialize"));
    }
    let target_where_clause = target_generics.where_clause.clone();
    let target_path = generic_path(&ast.ident, &target_generics);

    // The target type parameter is T: SameDeserialization<SameAs=Foo<'b0, T0, T1>>.
    let target_ty_param_bound = quote! { ::serde_roundtrip::SameDeserialization<SameAs=#target_path> };
    let target_ty_param = TyParam {
        attrs: vec![],
        ident: Ident::from("T"),
        bounds: vec![ty_param_bound(target_ty_param_bound.as_str())],
        default: None,
    };

    // If the original is Foo<'l, X, Y>, the source type is Foo<'a0, S0, S1>.
    let mut source_renaming = Renaming { original: &ast.generics, lifetime_prefix: "'a", ty_param_prefix: "S" };
    let mut source_generics = source_renaming.fold_generics(ast.generics.clone());
    for (ty_param, target_ty_param) in source_generics.ty_params.iter_mut().zip(target_generics.ty_params.iter()) {
        let target_ty_param_ident = &target_ty_param.ident;
        let text = quote! { ::serde_roundtrip::RoundTrip<#target_ty_param_ident> };
        ty_param.bounds.push(ty_param_bound(text.as_str()));
    }
    let source_path = generic_path(&ast.ident, &source_generics);

    // The whole thing is parameterized by 'a0, 'b0, S0, S1, T0, T1, T.
    let all_generics = Generics {
        lifetimes: source_generics.lifetimes.iter().cloned()
            .chain(target_generics.lifetimes.iter().cloned())
            .collect::<Vec<_>>(),
        ty_params: source_generics.ty_params.iter().cloned()
            .chain(target_generics.ty_params.iter().cloned())
            .chain(::std::iter::once(target_ty_param))
            .collect::<Vec<_>>(),
        where_clause: WhereClause {
            predicates: source_generics.where_clause.predicates.iter().cloned()
                .chain(target_generics.where_clause.predicates.iter().cloned())
                .collect::<Vec<_>>(),
        },
    };
    let all_where_clause = all_generics.where_clause.clone();

    // The recursive implementation of round_trip()

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

    // Implement RoundTrip and SameDeserialization

    quote! {
        impl #all_generics ::serde_roundtrip::RoundTrip<T> for #source_path
            #all_where_clause
        {
            fn round_trip(&self) -> T { T::from(#round_trip) }
        }
        impl #target_generics ::serde_roundtrip::SameDeserialization for #target_path
            #target_where_clause
        {
            type SameAs = Self;
            fn from(data: Self) -> Self { data }
        }
    }
}
