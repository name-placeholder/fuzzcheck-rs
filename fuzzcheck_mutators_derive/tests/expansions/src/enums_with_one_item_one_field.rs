use fuzzcheck_mutators::fuzzcheck_derive_mutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum X {
    A(u8),
}

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum Y {
    Y { y: Option<u8> },
}

