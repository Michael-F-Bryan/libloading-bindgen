use crate::bindings::{Bindings, ExternFunction};
use proc_macro2::{Span, TokenStream};
use syn::{
    token::{Brace, Paren},
    Abi, AngleBracketedGenericArguments, BareFnArg, Block, Expr, ExprCall,
    ExprMacro, ExprPath, ExprTry, Field, Fields, FieldsNamed, FnArg,
    GenericArgument, GenericParam, Generics, Ident, ImplItemMethod, Item,
    ItemImpl, ItemStruct, Local, Macro, MacroDelimiter, Pat, PatIdent, PatType,
    Path, PathArguments, PathSegment, PredicateType, ReturnType, Signature,
    Stmt, Token, TraitBound, TraitBoundModifier, Type, TypeBareFn, TypeParam,
    TypeParamBound, TypePath, VisPublic, Visibility, WhereClause,
    WherePredicate,
};

pub(crate) fn append_new_bindings(items: &mut Vec<Item>, bindings: Bindings) {
    items.push(bindings_vtable(&bindings).into());
    items.push(bindings_constructor(&bindings).into());
}

fn bindings_vtable(bindings: &Bindings) -> ItemStruct {
    let mut fields: Vec<Field> = Vec::new();

    for func in &bindings.functions {
        let sig = function_signature(func);

        fields.push(Field {
            attrs: Vec::new(),
            colon_token: Some(<Token!(:)>::default()),
            ident: Some(func.item.sig.ident.clone()),
            ty: Type::BareFn(sig),
            vis: Visibility::Inherited,
        });
    }

    ItemStruct {
        fields: Fields::Named(FieldsNamed {
            brace_token: Brace::default(),
            named: fields.into_iter().collect(),
        }),
        attrs: Vec::new(),
        generics: Generics::default(),
        ident: Ident::new("Bindings", Span::call_site()),
        vis: Visibility::Public(VisPublic {
            pub_token: <Token![pub]>::default(),
        }),
        semi_token: None,
        struct_token: <Token![struct]>::default(),
    }
}

fn function_signature(func: &ExternFunction) -> TypeBareFn {
    let sig = &func.item.sig;

    debug_assert_eq!(
        sig.generics.lifetimes().count(),
        0,
        "FFI functions shouldn't have associated lifetimes"
    );
    debug_assert!(
        sig.generics.params.is_empty(),
        "FFI bindings shouldn't be generic"
    );
    debug_assert!(sig.constness.is_none(), "FFI functions can't be `const`");
    debug_assert!(sig.asyncness.is_none(), "FFI functions can't be `async`");

    let Signature {
        inputs,
        output,
        variadic,
        fn_token,
        paren_token,
        ..
    } = sig;

    TypeBareFn {
        fn_token: fn_token.clone(),
        lifetimes: None,
        unsafety: Some(<Token![unsafe]>::default()),
        abi: Some(Abi {
            extern_token: <Token![extern]>::default(),
            name: func.abi.clone(),
        }),
        inputs: inputs.iter().map(to_bare_fn_arg).collect(),
        output: output.clone(),
        paren_token: paren_token.clone(),
        variadic: variadic.clone(),
    }
}

fn to_bare_fn_arg(arg: &FnArg) -> BareFnArg {
    match arg {
        FnArg::Receiver(_) => unreachable!(),
        FnArg::Typed(ty) => BareFnArg {
            attrs: Vec::new(),
            name: None,
            ty: (*ty.ty).clone(),
        },
    }
}

fn bindings_constructor(bindings: &Bindings) -> ItemImpl {
    let load_from_path = load_from_path(bindings);

    ItemImpl {
        attrs: Vec::new(),
        defaultness: None,
        unsafety: None,
        impl_token: Default::default(),
        generics: Generics::default(),
        trait_: None,
        self_ty: Box::new(Type::Path(TypePath {
            path: Path::from(PathSegment {
                ident: Ident::new("Bindings", Span::call_site()),
                arguments: PathArguments::None,
            }),
            qself: None,
        })),
        brace_token: Default::default(),
        items: vec![load_from_path.into()],
    }
}

fn load_from_path(bindings: &Bindings) -> ImplItemMethod {
    let sig = load_from_path_signature();

    let library_new = Expr::Call(ExprCall {
        func: Box::new(Expr::Path(ExprPath {
            path: Path {
                leading_colon: Some(<Token![::]>::default()),
                segments: vec![
                    PathSegment {
                        ident: Ident::new("libloading", Span::call_site()),
                        arguments: PathArguments::None,
                    },
                    PathSegment {
                        ident: Ident::new("Library", Span::call_site()),
                        arguments: PathArguments::None,
                    },
                    PathSegment {
                        ident: Ident::new("new", Span::call_site()),
                        arguments: PathArguments::None,
                    },
                ]
                .into_iter()
                .collect(),
            },
            attrs: Vec::new(),
            qself: None,
        })),
        attrs: Vec::new(),
        args: vec![Expr::Path(ExprPath {
            path: Path::from(PathSegment {
                ident: Ident::new("path", Span::call_site()),
                arguments: PathArguments::None,
            }),
            attrs: Vec::new(),
            qself: None,
        })]
        .into_iter()
        .collect(),
        paren_token: Paren::default(),
    });
    let opening_the_library = Stmt::Local(Local {
        pat: Pat::Ident(PatIdent {
            ident: Ident::new("library", Span::call_site()),
            attrs: Vec::new(),
            by_ref: None,
            mutability: None,
            subpat: None,
        }),
        init: Some((
            <Token![=]>::default(),
            Box::new(Expr::Try(ExprTry {
                expr: Box::new(library_new),
                attrs: Vec::new(),
                question_token: <Token![?]>::default(),
            })),
        )),
        attrs: Vec::new(),
        let_token: <Token![let]>::default(),
        semi_token: <Token![;]>::default(),
    });

    let block = Block {
        brace_token: Default::default(),
        stmts: vec![
            opening_the_library,
            Stmt::Expr(Expr::Macro(ExprMacro {
                attrs: Vec::new(),
                mac: Macro {
                    bang_token: Default::default(),
                    path: Path::from(PathSegment {
                        ident: Ident::new("todo", Span::call_site()),
                        arguments: PathArguments::None,
                    }),
                    tokens: TokenStream::new(),
                    delimiter: MacroDelimiter::Paren(Paren::default()),
                },
            })),
        ],
    };

    ImplItemMethod {
        attrs: Vec::new(),
        vis: Visibility::Public(VisPublic {
            pub_token: <Token![pub]>::default(),
        }),
        defaultness: None,
        sig,
        block,
    }
}

/// This is a really complicated way to write the following:
///
/// ```rust,ignore
/// fn load_from_path<P>(path: P) -> Result<Self, ::libloading::Error>
/// where
///   P: AsRef<::std::path::Path>
/// ```
fn load_from_path_signature() -> Signature {
    let libloading_error = TypePath {
        path: Path {
            leading_colon: Some(<Token![::]>::default()),
            segments: vec![
                PathSegment {
                    ident: Ident::new("libloading", Span::call_site()),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("Error", Span::call_site()),
                    arguments: PathArguments::None,
                },
            ]
            .into_iter()
            .collect(),
        },
        qself: None,
    };
    let capital_self = PathSegment {
        ident: Ident::new("Self", Span::call_site()),
        arguments: PathArguments::None,
    };

    let result_of_self_and_err = TypePath {
        path: Path::from(PathSegment {
            ident: Ident::new("Result", Span::call_site()),
            arguments: PathArguments::AngleBracketed(
                AngleBracketedGenericArguments {
                    colon2_token: Default::default(),
                    lt_token: Default::default(),
                    gt_token: Default::default(),
                    args: vec![
                        GenericArgument::Type(Type::Path(TypePath {
                            path: Path::from(capital_self),
                            qself: None,
                        })),
                        GenericArgument::Type(Type::Path(libloading_error)),
                    ]
                    .into_iter()
                    .collect(),
                },
            ),
        }),
        qself: None,
    };
    let output = ReturnType::Type(
        <Token![->]>::default(),
        Box::new(Type::Path(result_of_self_and_err)),
    );

    let as_ref_path = TypeParamBound::Trait(TraitBound {
        paren_token: None,
        modifier: TraitBoundModifier::None,
        lifetimes: None,
        path: Path::from(PathSegment {
            ident: Ident::new("AsRef", Span::call_site()),
            arguments: PathArguments::AngleBracketed(
                AngleBracketedGenericArguments {
                    colon2_token: Default::default(),
                    lt_token: Default::default(),
                    gt_token: Default::default(),
                    args: vec![GenericArgument::Type(Type::Path(TypePath {
                        path: Path {
                            segments: vec![
                                PathSegment::from(Ident::new(
                                    "std",
                                    Span::call_site(),
                                )),
                                PathSegment::from(Ident::new(
                                    "ffi",
                                    Span::call_site(),
                                )),
                                PathSegment::from(Ident::new(
                                    "OsStr",
                                    Span::call_site(),
                                )),
                            ]
                            .into_iter()
                            .collect(),
                            leading_colon: Some(<Token![::]>::default()),
                        },
                        qself: None,
                    }))]
                    .into_iter()
                    .collect(),
                },
            ),
        }),
    });

    let generics = Generics {
        lt_token: Some(<Token![<]>::default()),
        params: vec![GenericParam::Type(TypeParam {
            ident: Ident::new("P", Span::call_site()),
            attrs: Vec::new(),
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        })]
        .into_iter()
        .collect(),
        gt_token: Some(<Token![>]>::default()),
        where_clause: Some(WhereClause {
            where_token: <Token![where]>::default(),
            predicates: vec![WherePredicate::Type(PredicateType {
                lifetimes: None,
                colon_token: <Token![:]>::default(),
                bounded_ty: Type::Path(TypePath {
                    path: Path::from(PathSegment {
                        ident: Ident::new("P", Span::call_site()),
                        arguments: PathArguments::None,
                    }),
                    qself: None,
                }),
                bounds: vec![as_ref_path].into_iter().collect(),
            })]
            .into_iter()
            .collect(),
        }),
    };

    let inputs = vec![FnArg::Typed(PatType {
        attrs: Vec::new(),
        colon_token: <Token![:]>::default(),
        pat: Box::new(Pat::Ident(PatIdent {
            attrs: Vec::new(),
            by_ref: None,
            ident: Ident::new("path", Span::call_site()),
            mutability: None,
            subpat: None,
        })),
        ty: Box::new(Type::Path(TypePath {
            path: Path::from(PathSegment {
                ident: Ident::new("P", Span::call_site()),
                arguments: PathArguments::None,
            }),
            qself: None,
        })),
    })]
    .into_iter()
    .collect();

    Signature {
        constness: None,
        asyncness: None,
        unsafety: None,
        abi: None,
        fn_token: <Token![fn]>::default(),
        ident: Ident::new("load_from_path", Span::call_site()),
        generics,
        paren_token: Default::default(),
        inputs,
        variadic: None,
        output,
    }
}
