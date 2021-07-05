use crate::Common;
use crate::{decent_synquote_alternative::TokenBuilderExtend, MakeMutatorSettings};
use decent_synquote_alternative::{
    parser::{Generics, StructField, Ty, TypeParam, WhereClause},
    token_builder::TokenBuilder,
};
use proc_macro2::{Ident, Span, TokenStream};

// This file hosts the common code for generating default mutators for enums and structs

#[derive(Clone)]
pub struct FieldMutator {
    pub i: usize,
    pub j: Option<usize>,
    pub field: StructField,
    pub kind: FieldMutatorKind,
}

#[derive(Clone)]
pub enum FieldMutatorKind {
    Generic,
    Prescribed(Ty, Option<TokenStream>),
}
impl FieldMutator {
    pub(crate) fn mutator_stream(&self, cm: &Common) -> TokenStream {
        match &self.kind {
            FieldMutatorKind::Generic => {
                if let Some(j) = self.j {
                    ts!(cm.Mi_j.as_ref()(self.i, j))
                } else {
                    ts!(cm.Mi.as_ref()(self.i))
                }
            }
            FieldMutatorKind::Prescribed(m, _) => ts!(m),
        }
    }
}

#[allow(non_snake_case)]
pub(crate) struct CreateWrapperMutatorParams<'a> {
    pub(crate) cm: &'a Common,
    pub(crate) visibility: &'a TokenStream,
    pub(crate) type_ident: &'a Ident,
    pub(crate) type_generics: &'a Generics,
    pub(crate) type_where_clause: &'a Option<WhereClause>,
    pub(crate) field_mutators: &'a Vec<Vec<FieldMutator>>,
    pub(crate) InnerMutator: &'a TokenStream,
    pub(crate) new_impl: &'a TokenStream,
    pub(crate) default_impl: &'a TokenStream,
    pub(crate) settings: &'a MakeMutatorSettings,
}

#[allow(non_snake_case)]
pub(crate) fn make_mutator_type_and_impl(params: CreateWrapperMutatorParams) -> TokenStream {
    let CreateWrapperMutatorParams {
        cm,
        visibility,
        type_ident,
        type_generics,
        type_where_clause,
        field_mutators,
        InnerMutator,
        new_impl,
        default_impl,
        settings,
    } = params;

    let NameMutator = if let Some(name) = &settings.name {
        name.clone()
    } else {
        ident!(type_ident "Mutator")
    };

    let field_generic_mutators = field_mutators
        .iter()
        .flatten()
        .filter(|m| match m.kind {
            FieldMutatorKind::Generic => true,
            FieldMutatorKind::Prescribed(_, _) => false,
        })
        .collect::<Vec<_>>();

    let mut NameMutator_generics = type_generics.removing_eq_type();
    for field_mutator in field_generic_mutators.iter() {
        NameMutator_generics.type_params.push(TypeParam {
            type_ident: field_mutator.mutator_stream(&cm),
            ..<_>::default()
        })
    }
    let mut NameMutator_where_clause = type_where_clause.clone().unwrap_or(WhereClause::default());
    NameMutator_where_clause.add_clause_items(ts!(
        join_ts!(&type_generics.type_params, ty_param,
            ty_param.type_ident ":" cm.Clone "+ 'static ,"
        )
        join_ts!(&field_generic_mutators, field_mutator,
            field_mutator.mutator_stream(&cm) ":" cm.fuzzcheck_traits_Mutator "<" field_mutator.field.ty "> ,"
        )
    ));

    let field_prescribed_mutators = field_mutators
        .iter()
        .flatten()
        .filter_map(|m| match &m.kind {
            FieldMutatorKind::Generic => None,
            FieldMutatorKind::Prescribed(mutator, init) => Some((m.clone(), mutator.clone(), init.clone())),
        })
        .collect::<Vec<_>>();

    let mut Default_where_clause = NameMutator_where_clause.clone();
    Default_where_clause.add_clause_items(join_ts!(field_mutators.iter().flatten(), field_mutator,
        field_mutator.mutator_stream(&cm) ":" cm.Default
    , separator: ","));

    let mut DefaultMutator_Mutator_generics = type_generics.removing_bounds_and_eq_type();
    for field_mutator in field_mutators.iter().flatten() {
        match &field_mutator.kind {
            FieldMutatorKind::Generic => DefaultMutator_Mutator_generics.type_params.push(TypeParam {
                type_ident: ts!("<" field_mutator.field.ty "as" cm.DefaultMutator ">::Mutator"),
                ..<_>::default()
            }),
            FieldMutatorKind::Prescribed(_, _) => {}
        }
    }

    let mut DefaultMutator_where_clause = type_where_clause.clone().unwrap_or(WhereClause::default());
    DefaultMutator_where_clause.add_clause_items(ts!(
        join_ts!(&type_generics.type_params, ty_param,
            ty_param.type_ident ":" cm.DefaultMutator "+ 'static ,"
        )
        join_ts!(field_prescribed_mutators.iter().filter(|(_, _, init)| init.is_none()), (_, mutator, _),
            mutator ":" cm.Default ","
        )
    ));

    let NameMutatorCache = ident!(NameMutator "Cache");
    let NameMutatorMutationStep = ident!(NameMutator "MutationStep");
    let NameMutatorArbitraryStep = ident!(NameMutator "ArbitraryStep");
    let NameMutatorUnmutateToken = ident!(NameMutator "UnmutateToken");

    let helper_type = |helper_type: &str| {
        ts!(
            visibility "struct" ident!(NameMutator helper_type) NameMutator_generics.removing_eq_type() NameMutator_where_clause "{
            inner : "
                if settings.recursive {
                    ts!(cm.Box "<")
                } else {
                    ts!("")
                }
                "<" InnerMutator " as " cm.fuzzcheck_traits_Mutator "<" type_ident type_generics.removing_bounds_and_eq_type() "> >::" helper_type
                if settings.recursive {
                    ">"
                } else {
                    ""
                }
                ",
            }
            impl " NameMutator_generics.removing_eq_type() ident!(NameMutator helper_type) NameMutator_generics.removing_bounds_and_eq_type() NameMutator_where_clause "{
                fn new(inner: <" InnerMutator " as " cm.fuzzcheck_traits_Mutator "<" type_ident type_generics.removing_bounds_and_eq_type() "> >::" helper_type") -> Self {"
                    "Self {
                        inner: "  if settings.recursive { ts!(cm.Box "::new") } else { ts!("") }
                            "(inner)"
                        "
                    }"
                "}
            } 
            ")
    };

    let InnerMutator_as_Mutator = ts!("<" InnerMutator "as" cm.fuzzcheck_traits_Mutator "<" type_ident type_generics.removing_bounds_and_eq_type() "> >" );

    ts!(
    visibility "struct" NameMutator NameMutator_generics NameMutator_where_clause
    "{
        pub mutator:" InnerMutator "
    }"
    helper_type("Cache")
    helper_type("MutationStep")
    helper_type("ArbitraryStep")
    helper_type("UnmutateToken")

    "impl " NameMutator_generics NameMutator NameMutator_generics.removing_bounds_and_eq_type() NameMutator_where_clause "
    {"
        new_impl
    "}"
    "impl " NameMutator_generics cm.Default "for" NameMutator NameMutator_generics.removing_bounds_and_eq_type()
        Default_where_clause "
        {"
            default_impl
        "
        }
        impl " NameMutator_generics cm.fuzzcheck_traits_Mutator "<" type_ident type_generics.removing_bounds_and_eq_type() "> 
            for " NameMutator NameMutator_generics.removing_bounds_and_eq_type() NameMutator_where_clause "
        {
            type Cache =" NameMutatorCache NameMutator_generics.removing_bounds_and_eq_type() ";
            type MutationStep =" NameMutatorMutationStep NameMutator_generics.removing_bounds_and_eq_type() ";
            type ArbitraryStep =" NameMutatorArbitraryStep NameMutator_generics.removing_bounds_and_eq_type() ";
            type UnmutateToken =" NameMutatorUnmutateToken NameMutator_generics.removing_bounds_and_eq_type() ";
        
            
            fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
                Self::ArbitraryStep::new(" InnerMutator_as_Mutator "::default_arbitrary_step(&self.mutator))
            }

            
            fn validate_value(&self, value: &" type_ident type_generics.removing_bounds_and_eq_type() ") -> " cm.Option "<(Self::Cache, Self::MutationStep)> {
                if let " cm.Some "((c, s)) = " InnerMutator_as_Mutator "::validate_value(&self.mutator, value) {
                    " cm.Some "((Self::Cache::new(c) , Self::MutationStep::new(s) ))
                } else {
                    " cm.None "
                }
            }
            
            fn max_complexity(&self) -> f64 {
                " InnerMutator_as_Mutator "::max_complexity(&self.mutator)
            }
            
            fn min_complexity(&self) -> f64 {
                " InnerMutator_as_Mutator "::min_complexity(&self.mutator)
            }
            
            fn complexity(&self, value: &" type_ident type_generics.removing_bounds_and_eq_type() ", cache: &Self::Cache) -> f64 {
                " InnerMutator_as_Mutator "::complexity(&self.mutator, value, &cache.inner)
            }
            
            fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(" type_ident type_generics.removing_bounds_and_eq_type() ", f64)> {
                if let " cm.Some "((value, cplx)) = " InnerMutator_as_Mutator "::ordered_arbitrary(&self.mutator, &mut step.inner, max_cplx) {"
                cm.Some "((value, cplx))"
            "} else {"
                cm.None
            "}
            }
            
            fn random_arbitrary(&self, max_cplx: f64) -> (" type_ident type_generics.removing_bounds_and_eq_type() ", f64) {
                let (value, cplx) = " InnerMutator_as_Mutator "::random_arbitrary(&self.mutator, max_cplx) ;
                (value, cplx)
            }
            
            fn ordered_mutate(
                &self,
                value: &mut " type_ident type_generics.removing_bounds_and_eq_type() ",
                cache: &Self::Cache,
                step: &mut Self::MutationStep,
                max_cplx: f64,
            ) -> Option<(Self::UnmutateToken, f64)> {
                if let " cm.Some "((t, c)) = " InnerMutator_as_Mutator "::ordered_mutate(
                    &self.mutator,
                    value,
                    &cache.inner,
                    &mut step.inner,
                    max_cplx,
                ) {
                    " cm.Some "((Self::UnmutateToken::new(t), c))
                } else {"
                cm.None
            "}
            }
            
            fn random_mutate(&self, value: &mut " type_ident type_generics.removing_bounds_and_eq_type() ", cache: &Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
                let (t, c) =" InnerMutator_as_Mutator "::random_mutate(&self.mutator, value, &cache.inner, max_cplx);
                (Self::UnmutateToken::new(t), c)
            }
            
            fn unmutate(&self, value: &mut " type_ident type_generics.removing_bounds_and_eq_type() ", t: Self::UnmutateToken) {
                " InnerMutator_as_Mutator "::unmutate(&self.mutator, value, " if settings.recursive {
                "*t.inner"
            } else {
                "t.inner"
            }")
            }
        }"
        if settings.default {
            ts!("impl" type_generics.removing_eq_type() cm.DefaultMutator "for" type_ident type_generics.removing_bounds_and_eq_type() DefaultMutator_where_clause "{"
            if settings.recursive {
                ts!("type Mutator = " cm.RecursiveMutator "<" NameMutator DefaultMutator_Mutator_generics ">;")
            } else {
                ts!("type Mutator = "  NameMutator DefaultMutator_Mutator_generics ";")
            }
            "fn default_mutator() -> Self::Mutator {"
                if settings.recursive {
                    format!("{}::new(|self_| {{", cm.RecursiveMutator)
                } else {
                    "".to_string()
                }
                NameMutator "::new("
                    join_ts!(field_mutators.iter().flatten(), field_mutator,
                        match &field_mutator.kind {
                            FieldMutatorKind::Generic => {
                                ts!("<" field_mutator.field.ty "as" cm.DefaultMutator ">::default_mutator()")
                            }
                            FieldMutatorKind::Prescribed(_, Some(init)) => {
                                ts!("{" init "}")
                            }
                            FieldMutatorKind::Prescribed(mutator, None) => {
                                ts!("<" mutator "as" cm.Default ">::default()")
                            }
                        }
                    , separator: ",")
                ")"
                if settings.recursive {
                    "})"
                } else {
                    ""
                }
                "}
            }")
        } else {
            ts!()
        }
    )
}