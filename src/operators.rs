use crate::std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Sub};

use num_traits::{zero, Num};

use crate::{
    accum, compose, delay, fanout, identity, lift, map2, split, Compose, Fanout, Lift, SignalTrans,
    Split, SF,
};

impl<Time: Copy, T, U> BitOr<SF<Time, U>> for SF<Time, T>
where
    T: SignalTrans<Time>,
    U: SignalTrans<Time, Input = T::Output>,
{
    type Output = SF<Time, Compose<T, U>>;

    fn bitor(self, other: SF<Time, U>) -> Self::Output {
        compose(self, other)
    }
}

impl<Time: Copy, T, U> BitAnd<SF<Time, U>> for SF<Time, T>
where
    T: SignalTrans<Time>,
    U: SignalTrans<Time, Input = T::Input>,
    T::Input: Clone,
{
    type Output = SF<Time, Fanout<Time, T, U>>;

    fn bitand(self, other: SF<Time, U>) -> Self::Output {
        fanout(self, other)
    }
}

impl<Time: Copy, T, U> BitXor<SF<Time, U>> for SF<Time, T>
where
    T: SignalTrans<Time>,
    U: SignalTrans<Time>,
{
    type Output = SF<Time, Split<T, U>>;

    fn bitxor(self, other: SF<Time, U>) -> Self::Output {
        split(self, other)
    }
}

macro_rules! overload_operator {
    ($optrait:ident, $opfunc:ident, $op:tt) => {
        impl<Time: Copy, T, U> $optrait<SF<Time, U>> for SF<Time, T>
        where
            T: SignalTrans<Time>,
            U: SignalTrans<Time, Input = T::Input>,
            T::Input: Clone,
            T::Output: $optrait<U::Output>,
        {
            // TODO File bug report for impl ... working here
            type Output = SF<Time,
                Compose<
                    Fanout<Time, T, U>,
                    Lift<
                        Time,
                        (T::Output, U::Output),
                        <T::Output as $optrait<U::Output>>::Output,
                    >,
                >
            >;

            fn $opfunc(self, other: SF<Time, U>) -> Self::Output {
                (self & other) | map2(|x, y| x $op y)
            }
        }
    }
}

overload_operator!(Add, add, +);
overload_operator!(Sub, sub, -);
overload_operator!(Mul, mul, *);
overload_operator!(Div, div, /);

pub fn derivative<Time, T>() -> SF<Time, impl SignalTrans<Time, Input = T, Output = T>>
where
    Time: Num + Copy + Into<T>,
    T: Num + Clone,
{
    (identity::<Time, T>() - delay::<Time, T>(zero())) | lift(|dt: Time, df| df / dt.into())
}

pub fn integral<Time, T>() -> SF<Time, impl SignalTrans<Time, Input = T, Output = T>>
where
    T: Num + Clone,
    Time: Num + Into<T>,
{
    accum(zero(), |dt: Time, x, sum| sum + x * dt.into())
}
