use syn::{
    visit_mut::{self, VisitMut},
    File, ForeignItem, ForeignItemFn, ForeignItemStatic, Item, ItemForeignMod,
    LitStr,
};

pub(crate) fn extract_raw_bindings(file: &mut File) -> Bindings {
    // extract the items we care about
    let mut generator = Generator::default();

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

#[derive(Debug, Default)]
struct Generator {
    functions: Vec<ExternFunction>,
    statics: Vec<ForeignItemStatic>,
    current_abi: Option<LitStr>,
}

impl VisitMut for Generator {
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
                ForeignItem::Fn(item) => self.functions.push(ExternFunction {
                    abi: current_abi.clone(),
                    item,
                }),
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

        let bindings = extract_raw_bindings(&mut file);

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
