use crate::bindings::{Bindings, ExternFunction};
use proc_macro2::Span;
use syn::{
    token::{Brace, Paren},
    Abi, AngleBracketedGenericArguments, BareFnArg, Block, Expr, ExprCall,
    ExprLit, ExprMethodCall, ExprPath, ExprStruct, ExprTry, ExprUnary, Field,
    FieldValue, Fields, FieldsNamed, FnArg, GenericArgument, GenericParam,
    Generics, Ident, ImplItemMethod, Item, ItemImpl, ItemStruct, Lit,
    LitByteStr, Local, Pat, PatIdent, PatType, Path, PathArguments,
    PathSegment, PredicateType, ReturnType, Signature, Stmt, Token, TraitBound,
    TraitBoundModifier, Type, TypeBareFn, TypeParam, TypeParamBound, TypePath,
    UnOp, VisPublic, Visibility, WhereClause, WherePredicate,
};

pub(crate) fn append_new_bindings(items: &mut Vec<Item>, bindings: Bindings) {
    items.push(bindings_vtable(&bindings).into());
    items.push(bindings_constructor(&bindings).into());
}

fn bindings_vtable(bindings: &Bindings) -> ItemStruct {
    let mut fields: Vec<Field> = Vec::new();

    fields.push(Field {
        attrs: Vec::new(),
        colon_token: Some(<Token!(:)>::default()),
        ident: Some(Ident::new("library", Span::call_site())),
        ty: Type::Path(TypePath {
            path: Path {
                leading_colon: Some(<Token![::]>::default()),
                ..long_path(&["libloading", "Library"])
            },
            qself: None,
        }),
        vis: Visibility::Inherited,
    });

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
            path: short_path("Bindings"),
            qself: None,
        })),
        brace_token: Default::default(),
        items: vec![load_from_path.into()],
    }
}

fn long_path<I, S>(segments: I) -> Path
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let segments = segments
        .into_iter()
        .map(|segment| PathSegment {
            ident: Ident::new(segment.as_ref(), Span::call_site()),
            arguments: PathArguments::None,
        })
        .collect();

    Path {
        leading_colon: None,
        segments,
    }
}

fn short_path<S>(name: S) -> Path
where
    S: AsRef<str>,
{
    Path::from(PathSegment {
        ident: Ident::new(name.as_ref(), Span::call_site()),
        arguments: PathArguments::None,
    })
}

fn load_from_path(bindings: &Bindings) -> ImplItemMethod {
    let sig = load_from_path_signature();

    let library_new = Expr::Call(ExprCall {
        func: Box::new(Expr::Path(ExprPath {
            path: Path {
                leading_colon: Some(<Token![::]>::default()),
                ..long_path(&["libloading", "Library", "new"])
            },
            attrs: Vec::new(),
            qself: None,
        })),
        attrs: Vec::new(),
        args: vec![Expr::Path(ExprPath {
            path: short_path("path"),
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

    let mut stmts = vec![opening_the_library];
    let library_variable = ExprPath {
        path: short_path("library"),
        attrs: Vec::new(),
        qself: None,
    };

    let mut binding_struct_fields = vec![FieldValue {
        colon_token: None,
        member: syn::Member::Named(Ident::new("library", Span::call_site())),
        expr: Expr::Path(ExprPath {
            path: short_path("library"),
            attrs: Vec::new(),
            qself: None,
        }),
        attrs: Vec::new(),
    }];

    for func in &bindings.functions {
        let argument = func.item.sig.ident.to_string();

        let library_get = Expr::MethodCall(ExprMethodCall {
            attrs: Vec::new(),
            receiver: Box::new(Expr::Path(library_variable.clone())),
            dot_token: Default::default(),
            method: Ident::new("get", Span::call_site()),
            turbofish: None,
            paren_token: Default::default(),
            args: vec![Expr::Lit(ExprLit {
                lit: Lit::ByteStr(LitByteStr::new(
                    argument.as_bytes(),
                    Span::call_site(),
                )),
                attrs: Vec::new(),
            })]
            .into_iter()
            .collect(),
        });

        let assignment = Stmt::Local(Local {
            pat: Pat::Ident(PatIdent {
                ident: Ident::new(&argument, Span::call_site()),
                attrs: Vec::new(),
                by_ref: None,
                mutability: None,
                subpat: None,
            }),
            init: Some((
                <Token![=]>::default(),
                Box::new(Expr::Unary(ExprUnary {
                    attrs: Vec::new(),
                    op: UnOp::Deref(Default::default()),
                    expr: Box::new(Expr::Try(ExprTry {
                        expr: Box::new(library_get),
                        attrs: Vec::new(),
                        question_token: <Token![?]>::default(),
                    })),
                })),
            )),
            attrs: Vec::new(),
            let_token: <Token![let]>::default(),
            semi_token: <Token![;]>::default(),
        });

        stmts.push(assignment);
        binding_struct_fields.push(FieldValue {
            colon_token: None,
            member: syn::Member::Named(Ident::new(
                &argument,
                Span::call_site(),
            )),
            expr: Expr::Path(ExprPath {
                path: Path::from(PathSegment {
                    ident: Ident::new(&argument, Span::call_site()),
                    arguments: PathArguments::None,
                }),
                attrs: Vec::new(),
                qself: None,
            }),
            attrs: Vec::new(),
        });
    }

    let binding_struct_literal = Expr::Struct(ExprStruct {
        path: short_path("Bindings"),
        fields: binding_struct_fields.into_iter().collect(),
        brace_token: Default::default(),
        dot2_token: None,
        rest: None,
        attrs: Vec::new(),
    });
    stmts.push(Stmt::Expr(Expr::Call(ExprCall {
        func: Box::new(Expr::Path(ExprPath {
            path: short_path("Ok"),
            attrs: Vec::new(),
            qself: None,
        })),
        args: vec![binding_struct_literal].into_iter().collect(),
        paren_token: Default::default(),
        attrs: Vec::new(),
    })));

    let block = Block {
        brace_token: Default::default(),
        stmts,
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

fn generic_type<A>(name: &str, args: A) -> Path
where
    A: IntoIterator<Item = Path>,
{
    Path::from(PathSegment {
        ident: Ident::new(name, Span::call_site()),
        arguments: PathArguments::AngleBracketed(
            AngleBracketedGenericArguments {
                colon2_token: Default::default(),
                lt_token: Default::default(),
                gt_token: Default::default(),
                args: args
                    .into_iter()
                    .map(|path| {
                        GenericArgument::Type(Type::Path(TypePath {
                            path,
                            qself: None,
                        }))
                    })
                    .collect(),
            },
        ),
    })
}

/// This is a really complicated way to write the following:
///
/// ```rust,ignore
/// fn load_from_path<P>(path: P) -> Result<Self, ::libloading::Error>
/// where
///   P: AsRef<::std::path::Path>
/// ```
fn load_from_path_signature() -> Signature {
    let libloading_error = Path {
        leading_colon: Some(<Token![::]>::default()),
        ..long_path(&["libloading", "Error"])
    };

    let result_of_self_and_err = TypePath {
        path: generic_type(
            "Result",
            vec![short_path("Self"), libloading_error],
        ),
        qself: None,
    };
    let output = ReturnType::Type(
        <Token![->]>::default(),
        Box::new(Type::Path(result_of_self_and_err)),
    );

    let as_ref_osstr = TypeParamBound::Trait(TraitBound {
        paren_token: None,
        modifier: TraitBoundModifier::None,
        lifetimes: None,
        path: generic_type(
            "AsRef",
            vec![Path {
                leading_colon: Some(<Token![::]>::default()),
                ..long_path(&["std", "ffi", "OsStr"])
            }],
        ),
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
                    path: short_path("P"),
                    qself: None,
                }),
                bounds: vec![as_ref_osstr].into_iter().collect(),
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
            path: short_path("P"),
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
