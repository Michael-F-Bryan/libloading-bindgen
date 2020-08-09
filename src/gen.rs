use crate::{
    bindings::{Bindings, ExternFunction},
    BindingStrategy,
};
use proc_macro2::Span;
use syn::{
    token::Brace, Abi, BareFnArg, Field, Fields, FieldsNamed, FnArg, Generics,
    Ident, Item, ItemStruct, Signature, Token, Type, TypeBareFn, VisPublic,
    Visibility,
};

pub(crate) fn append_new_bindings<S>(
    items: &mut Vec<Item>,
    bindings: Bindings,
    strategy: &S,
) where
    S: BindingStrategy,
{
    let vtable = bindings_vtable(&bindings, strategy);
    items.push(vtable.into());
}

fn bindings_vtable<S>(bindings: &Bindings, strategy: &S) -> ItemStruct
where
    S: BindingStrategy,
{
    let mut fields: Vec<Field> = Vec::new();

    for func in &bindings.functions {
        if strategy.should_include(&func.item) {
            let sig = function_signature(func);

            fields.push(Field {
                attrs: Vec::new(),
                colon_token: Some(<Token!(:)>::default()),
                ident: Some(func.item.sig.ident.clone()),
                ty: Type::BareFn(sig),
                vis: Visibility::Inherited,
            });
        }
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
