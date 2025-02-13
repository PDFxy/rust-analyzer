//! Attributes & documentation for hir types.
use hir_def::{
    attr::{AttrsWithOwner, Documentation},
    path::ModPath,
    per_ns::PerNs,
    resolver::HasResolver,
    AttrDefId, GenericParamId, ModuleDefId,
};
use hir_expand::hygiene::Hygiene;
use hir_ty::db::HirDatabase;
use syntax::ast;

use crate::{
    Adt, Const, ConstParam, Enum, Field, Function, GenericParam, Impl, LifetimeParam, MacroDef,
    Module, ModuleDef, Static, Struct, Trait, TypeAlias, TypeParam, Union, Variant,
};

pub trait HasAttrs {
    fn attrs(self, db: &dyn HirDatabase) -> AttrsWithOwner;
    fn docs(self, db: &dyn HirDatabase) -> Option<Documentation>;
    fn resolve_doc_path(
        self,
        db: &dyn HirDatabase,
        link: &str,
        ns: Option<Namespace>,
    ) -> Option<ModuleDef>;
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum Namespace {
    Types,
    Values,
    Macros,
}

macro_rules! impl_has_attrs {
    ($(($def:ident, $def_id:ident),)*) => {$(
        impl HasAttrs for $def {
            fn attrs(self, db: &dyn HirDatabase) -> AttrsWithOwner {
                let def = AttrDefId::$def_id(self.into());
                db.attrs(def)
            }
            fn docs(self, db: &dyn HirDatabase) -> Option<Documentation> {
                let def = AttrDefId::$def_id(self.into());
                db.attrs(def).docs()
            }
            fn resolve_doc_path(self, db: &dyn HirDatabase, link: &str, ns: Option<Namespace>) -> Option<ModuleDef> {
                let def = AttrDefId::$def_id(self.into());
                resolve_doc_path(db, def, link, ns).map(ModuleDef::from)
            }
        }
    )*};
}

impl_has_attrs![
    (Field, FieldId),
    (Variant, EnumVariantId),
    (Static, StaticId),
    (Const, ConstId),
    (Trait, TraitId),
    (TypeAlias, TypeAliasId),
    (MacroDef, MacroDefId),
    (Function, FunctionId),
    (Adt, AdtId),
    (Module, ModuleId),
    (GenericParam, GenericParamId),
    (Impl, ImplId),
];

macro_rules! impl_has_attrs_enum {
    ($($variant:ident),* for $enum:ident) => {$(
        impl HasAttrs for $variant {
            fn attrs(self, db: &dyn HirDatabase) -> AttrsWithOwner {
                $enum::$variant(self).attrs(db)
            }
            fn docs(self, db: &dyn HirDatabase) -> Option<Documentation> {
                $enum::$variant(self).docs(db)
            }
            fn resolve_doc_path(self, db: &dyn HirDatabase, link: &str, ns: Option<Namespace>) -> Option<ModuleDef> {
                $enum::$variant(self).resolve_doc_path(db, link, ns)
            }
        }
    )*};
}

impl_has_attrs_enum![Struct, Union, Enum for Adt];
impl_has_attrs_enum![TypeParam, ConstParam, LifetimeParam for GenericParam];

fn resolve_doc_path(
    db: &dyn HirDatabase,
    def: AttrDefId,
    link: &str,
    ns: Option<Namespace>,
) -> Option<ModuleDefId> {
    let resolver = match def {
        AttrDefId::ModuleId(it) => it.resolver(db.upcast()),
        AttrDefId::FieldId(it) => it.parent.resolver(db.upcast()),
        AttrDefId::AdtId(it) => it.resolver(db.upcast()),
        AttrDefId::FunctionId(it) => it.resolver(db.upcast()),
        AttrDefId::EnumVariantId(it) => it.parent.resolver(db.upcast()),
        AttrDefId::StaticId(it) => it.resolver(db.upcast()),
        AttrDefId::ConstId(it) => it.resolver(db.upcast()),
        AttrDefId::TraitId(it) => it.resolver(db.upcast()),
        AttrDefId::TypeAliasId(it) => it.resolver(db.upcast()),
        AttrDefId::ImplId(it) => it.resolver(db.upcast()),
        AttrDefId::GenericParamId(it) => match it {
            GenericParamId::TypeParamId(it) => it.parent,
            GenericParamId::LifetimeParamId(it) => it.parent,
            GenericParamId::ConstParamId(it) => it.parent,
        }
        .resolver(db.upcast()),
        AttrDefId::MacroDefId(_) => return None,
    };
    let path = ast::Path::parse(link).ok()?;
    let modpath = ModPath::from_src(db.upcast(), path, &Hygiene::new_unhygienic()).unwrap();
    let resolved = resolver.resolve_module_path_in_items(db.upcast(), &modpath);
    let resolved = if resolved == PerNs::none() {
        resolver.resolve_module_path_in_trait_assoc_items(db.upcast(), &modpath)?
    } else {
        resolved
    };
    match ns {
        Some(Namespace::Types) => resolved.take_types(),
        Some(Namespace::Values) => resolved.take_values(),
        Some(Namespace::Macros) => None,
        None => resolved.iter_items().find_map(|it| it.as_module_def_id()),
    }
}
