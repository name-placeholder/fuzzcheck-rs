use crate::DefaultMutator;
use fuzzcheck_traits::Mutator;

/*
    These mutators try to achieve multiple things:
    * avoid repetitions, such that if the value “7” was already produced, then it will not appear again
    * cover most of the search space as quickly as possible. For example, for 8-bit unsigned integers,
      it is good to produce a sequence such as: 0, 255, 128, 192, 64, 224, 32, etc.
    * also produce values close to the original integer first. So mutating 100 will first produce numbers
      such as 101, 99, 102, 98, etc.
    * be very fast

    One idea to create arbitrary integers that don't repeat themselves and span the whole search space was
    to use a binary-search-like approach, as written in the function binary_search_arbitrary. However that
    turns out to be quite slow for an integer mutator. So what I do instead is create a sequence of 256 integers
    using that approach, which I store in a variable called “shuffled integers”. So shuffled integers is a
    vector that look something like this:
        [0, 256, 128, 192, 64, 224, 32, ...]
    Now, for an 8-bit integer type, it is enough to simply index that vector to get an arbitrary value that
    respects all the criteria I laid above. But for types that have 16, 32, 64, or 128 bits, I can't do that.
    So I index the shuffled_integers vector multiple times until I have all the bits I need. For an u32, I need
    to index it four times. It is done in the following way:
    1. first I look at the lower 8 bits of steps to get the first index
        * so if step == 67, then I use the index 67
        * but if step == 259, then I use the index 3
    2. I get a number between 0 and 255 by getting shuffled_integers[first_index], I memorize that pick.
    3. I place the picked number on the highest 8 bits of the generated integer
        * so imagine the step was 259, then the index is 3 and shuffled_integers[3] is 192
        * so the generated integer, so far, is (192 << 24) == 3_221_225_472

    Let's stop to think about what that achieves. It means that for the first 256 steps, the
    8 highest bits of the generated integer will be [0, 256, 128, 192, ...]. So we are covering a huge part
    of the possible space in the first 256 steps alone. The goal is to use that strategy recursively for the
    remaining bits, while adding a little but of arbitrariness to it.

    4. Then I shift the step right by 8 bits. If it was 259 originally, it is now equal to (259 >> 8) == 3.
    5. And then I XOR that index with the the previous pick (the purpose of that
    is to make the generation a little bit more arbitrary/less predictable)
        * so the new index is (3 ^ 192) == 195
    6. I then get the next pick by getting shuffled_integers[192], let's say it is == 43.
    7. Then we update the generated integer, it is now (192 << 24) | (43 << 16)
    8. The next step is (259 >> 16) ^ 43 == 43
    9. etc.

    You can find more details on how it is done in `uniform_permutation`
*/

fn binary_search_arbitrary(low: u8, high: u8, step: u64) -> u8 {
    let next = low.wrapping_add(high.wrapping_sub(low) / 2);
    if low.wrapping_add(1) == high {
        if step % 2 == 0 {
            high
        } else {
            low
        }
    } else if step == 0 {
        next
    } else if step % 2 == 1 {
        binary_search_arbitrary(next.wrapping_add(1), high, step / 2)
    } else {
        // step % 2 == 0
        binary_search_arbitrary(low, next.wrapping_sub(1), (step - 1) / 2)
    }
}

macro_rules! impl_unsigned_mutator {
    ($name:ty,$name_mutator:ident,$rand:path,$size:expr) => {
        pub struct $name_mutator {
            shuffled_integers: [u8; 256],
            rng: fastrand::Rng,
        }
        impl Default for $name_mutator {
            fn default() -> Self {
                let mut shuffled_integers = [0; 256];
                for (i, x) in shuffled_integers.iter_mut().enumerate() {
                    *x = binary_search_arbitrary(0, u8::MAX, i as u64);
                }
                $name_mutator {
                    shuffled_integers,
                    rng: fastrand::Rng::default(),
                }
            }
        }

        impl $name_mutator {
            fn uniform_permutation(&self, step: u64) -> $name {
                let size = $size as u64;

                // granularity is the number of bits provided by shuffled_integers
                // in this case, it is fixed to 8 but I could use something different

                //                                  <- 64 bits for usize
                // 0000 ... 0000 0001 0000 0000     <- - 57 leading zeros for shuffled_integers.len()
                //                                  <- - 1
                //                                   =  8
                const GRANULARITY: u64 =
                    ((std::mem::size_of::<usize>() * 8) - (256u64.leading_zeros() as usize) - 1) as u64;

                const STEP_MASK: u64 = ((u8::MAX as usize) >> (8 - GRANULARITY)) as u64;
                // if I have a number, such as 983487234238, I can `AND` it with the step_mask
                // to get an index I can use on shuffled_integers.
                // in this case, the step_mask is fixed to
                // 0000 ... 0000 1111 1111
                // it gives a number between 0 and 256

                // step_i is used to index into shuffled_integers. The first value is the step
                // given as argument to this function.
                let step_i = (step & STEP_MASK) as usize;

                // now we start building the integer by taking bits from shuffled_integers
                // repeatedly. First by indexing it with step_i
                let mut prev = unsafe { *self.shuffled_integers.get_unchecked(step_i) as $name };

                // I put those bits at the highest  position, then I will fill in the lower bits
                let mut result = (prev << (size - GRANULARITY)) as $name;

                // remember, granulariy is the number of bits we fill in at a time
                // and size is the total size of the generated integer, in bits
                // For u64 and a granularity of 8, we get
                // for i in [1, 2, 3, 4, 5, 6, 7] { ... }
                for i in 1..(size / GRANULARITY) {
                    // each time, we shift step by `granularity` (e.g. 8) more bits to the right

                    // so, for a step of 167 and a granularity of 8, then the next step will be 0
                    // it's only after steps larger than 255 that the next step will be greater than 0

                    // and then we XOR it with previous integer picked from shuffled_integers[step_i]
                    // to get the next index into shuffled_integers, which we insert into
                    // the generated integer at the right place
                    let step_i = (((step >> (i * GRANULARITY)) ^ prev as u64) & STEP_MASK) as usize;
                    prev = unsafe { *self.shuffled_integers.get_unchecked(step_i) as $name };
                    result |= prev << (size - (i + 1) * GRANULARITY);
                }

                result
            }
        }

        impl Mutator for $name_mutator {
            type Value = $name;
            type Cache = ();
            type MutationStep = u64; // mutation step
            type ArbitraryStep = u64;
            type UnmutateToken = $name; // old value

            fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}

            fn initial_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {
                0
            }

            fn ordered_arbitrary(
                &mut self,
                step: &mut Self::ArbitraryStep,
                _max_cplx: f64,
            ) -> Option<(Self::Value, Self::Cache)> {
                if *step > <$name>::MAX as u64 {
                    None
                } else {
                    let value = self.uniform_permutation(*step);
                    *step += 1;
                    Some((value, ()))
                }
            }
            fn random_arbitrary(&mut self, _max_cplx: f64) -> (Self::Value, Self::Cache) {
                let value = self.uniform_permutation(self.rng.u64(..));
                (value, ())
            }

            fn max_complexity(&self) -> f64 {
                8.0
            }

            fn min_complexity(&self) -> f64 {
                8.0
            }

            fn complexity(&self, _value: &Self::Value, _cache: &Self::Cache) -> f64 {
                8.0
            }

            fn ordered_mutate(
                &mut self,
                value: &mut Self::Value,
                _cache: &mut Self::Cache,
                step: &mut Self::MutationStep,
                _max_cplx: f64,
            ) -> Option<Self::UnmutateToken> {
                if *step > 10u64.saturating_add(<$name>::MAX as u64) {
                    return None;
                }
                let token = *value;
                *value = {
                    let mut tmp_step = *step;
                    if tmp_step < 8 {
                        let nudge = (tmp_step + 2) as $name;
                        if nudge % 2 == 0 {
                            value.wrapping_add(nudge / 2)
                        } else {
                            value.wrapping_sub(nudge / 2)
                        }
                    } else {
                        tmp_step -= 7;
                        self.uniform_permutation(tmp_step)
                    }
                };
                *step = step.wrapping_add(1);

                Some(token)
            }
            fn random_mutate(
                &mut self,
                value: &mut Self::Value,
                _cache: &mut Self::Cache,
                _max_cplx: f64,
            ) -> Self::UnmutateToken {
                std::mem::replace(value, $rand(..))
            }

            fn unmutate(&self, value: &mut Self::Value, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
                *value = t;
            }
        }

        impl DefaultMutator for $name {
            type Mutator = $name_mutator;
            fn default_mutator() -> Self::Mutator {
                <$name_mutator>::default()
            }
        }
    };
}

impl_unsigned_mutator!(u8, U8Mutator, fastrand::u8, 8);
impl_unsigned_mutator!(u16, U16Mutator, fastrand::u16, 16);
impl_unsigned_mutator!(u32, U32Mutator, fastrand::u32, 32);
impl_unsigned_mutator!(u64, U64Mutator, fastrand::u64, 64);

macro_rules! impl_signed_mutator {
    ($name:ty,$name_unsigned:ty,$name_mutator:ident,$rand:path,$size:expr) => {
        pub struct $name_mutator {
            shuffled_integers: [u8; 256],
            rng: fastrand::Rng,
        }
        impl Default for $name_mutator {
            fn default() -> Self {
                let mut shuffled_integers = [0; 256];
                for (i, x) in shuffled_integers.iter_mut().enumerate() {
                    *x = binary_search_arbitrary(0, u8::MAX, i as u64);
                }
                $name_mutator {
                    shuffled_integers,
                    rng: fastrand::Rng::default(),
                }
            }
        }

        impl $name_mutator {
            fn uniform_permutation(&self, step: u64) -> $name_unsigned {
                let size = $size as u64;
                const GRANULARITY: u64 =
                    ((std::mem::size_of::<usize>() * 8) - (256_u64.leading_zeros() as usize) - 1) as u64;
                const STEP_MASK: u64 = ((u8::MAX as usize) >> (8 - GRANULARITY)) as u64;

                let step_i = (step & STEP_MASK) as usize;
                let mut prev = unsafe { *self.shuffled_integers.get_unchecked(step_i) as $name_unsigned };

                let mut result = (prev << (size - GRANULARITY)) as $name_unsigned;

                for i in 1..(size / GRANULARITY) {
                    let step_i = (((step >> (i * GRANULARITY)) ^ prev as u64) & STEP_MASK) as usize;
                    prev = unsafe { *self.shuffled_integers.get_unchecked(step_i) as $name_unsigned };
                    result |= prev << (size - (i + 1) * GRANULARITY);
                }

                result as $name_unsigned
            }
        }

        impl Mutator for $name_mutator {
            type Value = $name;
            type Cache = ();
            type MutationStep = u64; // mutation step
            type ArbitraryStep = u64;
            type UnmutateToken = $name; // old value

            fn cache_from_value(&self, _value: &Self::Value) -> Self::Cache {}
            fn initial_step_from_value(&self, _value: &Self::Value) -> Self::MutationStep {
                0
            }

            fn ordered_arbitrary(
                &mut self,
                step: &mut Self::ArbitraryStep,
                _max_cplx: f64,
            ) -> Option<(Self::Value, Self::Cache)> {
                if *step > <$name_unsigned>::MAX as u64 {
                    None
                } else {
                    let value = self.uniform_permutation(*step) as $name;
                    *step += 1;
                    Some((value, ()))
                }
            }
            fn random_arbitrary(&mut self, _max_cplx: f64) -> (Self::Value, Self::Cache) {
                let value = self.uniform_permutation(self.rng.u64(..)) as $name;
                (value, ())
            }

            fn max_complexity(&self) -> f64 {
                8.0
            }

            fn min_complexity(&self) -> f64 {
                8.0
            }

            fn complexity(&self, _value: &Self::Value, _cache: &Self::Cache) -> f64 {
                8.0
            }

            fn ordered_mutate(
                &mut self,
                value: &mut Self::Value,
                _cache: &mut Self::Cache,
                step: &mut Self::MutationStep,
                _max_cplx: f64,
            ) -> Option<Self::UnmutateToken> {
                let token = *value;
                *value = {
                    let mut tmp_step = *step;
                    if tmp_step < 8 {
                        let nudge = (tmp_step + 2) as $name;
                        if nudge % 2 == 0 {
                            value.wrapping_add(nudge / 2)
                        } else {
                            value.wrapping_sub(nudge / 2)
                        }
                    } else {
                        tmp_step -= 7;
                        self.uniform_permutation(tmp_step) as $name
                    }
                };
                *step = step.wrapping_add(1);

                Some(token)
            }
            fn random_mutate(
                &mut self,
                value: &mut Self::Value,
                _cache: &mut Self::Cache,
                _max_cplx: f64,
            ) -> Self::UnmutateToken {
                std::mem::replace(value, $rand(..))
            }

            fn unmutate(&self, value: &mut Self::Value, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
                *value = t;
            }
        }
    };
}

impl_signed_mutator!(i8, u8, I8Mutator, fastrand::i8, 8);
impl_signed_mutator!(i16, u16, I16Mutator, fastrand::i16, 16);
impl_signed_mutator!(i32, u32, I32Mutator, fastrand::i32, 32);
impl_signed_mutator!(i64, u64, I64Mutator, fastrand::i64, 64);
