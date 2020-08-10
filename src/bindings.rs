use crate::BindingStrategy;
use syn::{
    visit_mut::{self, VisitMut},
    File, ForeignItem, ForeignItemFn, ForeignItemStatic, Item, ItemForeignMod,
    LitStr,
};

pub(crate) fn extract_raw_bindings<S>(file: &mut File, strategy: &S) -> Bindings
where
    S: BindingStrategy,
{
    // extract the items we care about
    let mut generator = Generator::with_strategy(strategy);

    generator.visit_file_mut(file);

    let Generator {
        functions, statics, ..
    } = generator;

    Bindings { functions, statics }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Bindings {
    pub(crate) functions: Vec<ExternFunction>,
    pub(crate) statics: Vec<ForeignItemStatic>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExternFunction {
    pub(crate) abi: Option<LitStr>,
    pub(crate) item: ForeignItemFn,
}

#[derive(Debug)]
struct Generator<'a, S> {
    functions: Vec<ExternFunction>,
    statics: Vec<ForeignItemStatic>,
    current_abi: Option<LitStr>,
    strategy: &'a S,
}

impl<'a, S: BindingStrategy> Generator<'a, S> {
    fn with_strategy(strategy: &'a S) -> Self {
        Generator {
            strategy,
            functions: Vec::new(),
            statics: Vec::new(),
            current_abi: None,
        }
    }
}

impl<'a, S: BindingStrategy> VisitMut for Generator<'a, S> {
    fn visit_file_mut(&mut self, file: &mut File) {
        visit_mut::visit_file_mut(self, file);

        // make sure we don't emit empty `extern "C" {}` blocks
        file.items.retain(|item| match item {
            Item::ForeignMod(extern_block) if extern_block.items.is_empty() => {
                false
            },
            _ => true,
        });
    }

    fn visit_item_foreign_mod_mut(
        &mut self,
        extern_block: &mut ItemForeignMod,
    ) {
        for it in &mut extern_block.attrs {
            self.visit_attribute_mut(it)
        }
        self.visit_abi_mut(&mut extern_block.abi);

        let current_abi = &extern_block.abi.name;
        let items = std::mem::replace(&mut extern_block.items, Vec::new());

        for it in items.into_iter() {
            match it {
                ForeignItem::Fn(item) => {
                    if self.strategy.should_include(&item) {
                        self.functions.push(ExternFunction {
                            abi: current_abi.clone(),
                            item,
                        });
                    }
                },
                ForeignItem::Static(s) => self.statics.push(s),
                mut other => {
                    self.visit_foreign_item_mut(&mut other);
                    extern_block.items.push(other);
                },
            }
        }

        self.current_abi = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::Item;

    struct Always;

    impl BindingStrategy for Always {
        fn should_include(&self, _item: &ForeignItemFn) -> bool { true }
    }

    #[test]
    fn extract_bindings_from_rust_code() {
        let src = r#"
        #![allow(foo)]
        use std::os::raw::{c_int, c_char};

        extern "C" {
            static VERSION: *const c_char;

            fn add(left: c_int, right: c_int) -> c_int;
        }

        fn normal_rust() -> u32 { 42 }
        "#;
        let mut file: File = syn::parse_str(src).unwrap();

        let bindings = extract_raw_bindings(&mut file, &Always);

        assert_eq!(file.attrs, file.attrs);
        // the `extern "C"` block should have been removed
        assert_eq!(
            file.items,
            file.items
                .iter()
                .cloned()
                .filter(|it| !matches!(it, Item::ForeignMod(_)))
                .collect::<Vec<_>>()
        );
        assert_eq!(bindings.functions.len(), 1);
        assert_eq!(bindings.statics.len(), 1);
    }
}
