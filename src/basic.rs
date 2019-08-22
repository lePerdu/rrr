use crate::std::{marker::PhantomData, rc::Rc};

use crate::sf::{SignalTrans, SF};

//
// Identity
//

#[derive(Copy, Clone)]
pub struct Identity<A> {
    _a: PhantomData<A>,
}

impl<A> Default for Identity<A> {
    fn default() -> Self {
        Self {
            _a: Default::default(),
        }
    }
}

impl<A> Identity<A> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<Time, A> SignalTrans<Time> for Identity<A> {
    type Input = A;
    type Output = A;
    fn step(self, _: Time, a: A) -> (Self, A) {
        (self, a)
    }
}

pub fn identity<Time, A>() -> SF<Time, Identity<A>> {
    SF::from(Identity::new())
}

//
// Const
//

#[derive(Copy, Clone)]
pub struct Const<A, B> {
    _a: PhantomData<A>,
    value: B,
}

impl<A, B: Default> Default for Const<A, B> {
    fn default() -> Self {
        Self::from(B::default())
    }
}

impl<A, B> From<B> for Const<A, B> {
    fn from(value: B) -> Self {
        Const {
            _a: Default::default(),
            value: value,
        }
    }
}

impl<Time, A, B: Clone> SignalTrans<Time> for Const<A, B> {
    type Input = A;
    type Output = B;
    fn step(self, _: Time, _: A) -> (Self, B) {
        let b = self.value.clone();
        (self, b)
    }
}

pub fn constant<Time, A, B: Clone>(value: B) -> SF<Time, Const<A, B>> {
    SF::from(Const::from(value))
}

//
// LocalTime
//

#[derive(Copy, Clone, Default)]
pub struct LocalTime<Time> {
    time: Time,
}

//
// Lift
//

#[derive(Clone)]
pub struct Lift<Time, A, B>(Rc<dyn Fn(Time, A) -> B>);

impl<Time, A, B, F> From<F> for Lift<Time, A, B>
where
    F: Fn(Time, A) -> B + 'static,
{
    fn from(f: F) -> Self {
        Self(Rc::new(f))
    }
}

impl<Time, A, B> SignalTrans<Time> for Lift<Time, A, B> {
    type Input = A;
    type Output = B;

    fn step(self, delta: Time, a: A) -> (Self, B) {
        let b = (self.0)(delta, a);
        (self, b)
    }
}

macro_rules! make_lift {
    (
        $func:ident,
        $func_pure:ident,
        $result:ident,
        $($args:ident),+
    ) => {
        #[allow(unused_parens, non_snake_case)]
        pub fn $func<Time, $($args),*, $result, F>(f: F)
        -> SF<Time, Lift<Time, ($($args),*), $result>>
        where F: Fn(Time, $($args),*) -> $result + 'static {
            SF::from(Lift::from(move |delta, ($($args),*)| f(delta, $($args),*)))
        }

        #[allow(unused_parens, non_snake_case)]
        pub fn $func_pure<Time, $($args),*, $result, F>(f: F)
        -> SF<Time, Lift<Time, ($($args),*), $result>>
        where F: Fn($($args),*) -> $result + 'static {
            SF::from(Lift::from(move |_, ($($args),*)| f($($args),*)))
        }
    }
}

// TODO map -> lift and lift -> lift_time / lift_with_time?
// NOTE These really arn't necessary since all they do is spread tuples, but
// they can shorten code in some cases (i.e. lifting std functions)
make_lift!(lift, map, R, A);
make_lift!(lift2, map2, R, A, B);
make_lift!(lift3, map3, R, A, B, C);
make_lift!(lift4, map4, R, A, B, C, D);

//
// Compose
//

#[derive(Copy, Clone)]
pub struct Compose<T, U> {
    left: T,
    right: U,
}

impl<T, U> Compose<T, U> {
    pub fn new(left: T, right: U) -> Self {
        Self {
            left: left,
            right: right,
        }
    }
}

impl<Time, T, U> SignalTrans<Time> for Compose<T, U>
where
    Time: Copy,
    T: SignalTrans<Time>,
    U: SignalTrans<Time, Input = T::Output>,
{
    type Input = T::Input;
    type Output = U::Output;

    fn step(self, delta: Time, a: Self::Input) -> (Self, Self::Output) {
        let Compose {
            left: t, right: u, ..
        } = self;

        let (t_next, b) = t.step(delta, a);
        let (u_next, c) = u.step(delta, b);

        (Compose::new(t_next, u_next), c)
    }
}

pub fn compose<Time, T, U>(
    left: SF<Time, T>,
    right: SF<Time, U>,
) -> SF<Time, Compose<T, U>>
where
    Time: Copy,
    T: SignalTrans<Time>,
    U: SignalTrans<Time, Input = T::Output>,
{
    SF::from(Compose::new(left.into_inner(), right.into_inner()))
}

//
// Split
//

#[derive(Clone)]
pub struct Split<T, U> {
    first: T,
    second: U,
}

impl<T, U> Split<T, U> {
    pub fn new(first: T, second: U) -> Self {
        Self {
            first: first,
            second: second,
        }
    }
}

impl<Time: Copy, T, U> SignalTrans<Time> for Split<T, U>
where
    T: SignalTrans<Time>,
    U: SignalTrans<Time>,
{
    type Input = (T::Input, U::Input);
    type Output = (T::Output, U::Output);

    fn step(self, delta: Time, (a, b): Self::Input) -> (Self, Self::Output) {
        let Split { first, second } = self;
        let (first_next, a_next) = first.step(delta, a);
        let (second_next, b_next) = second.step(delta, b);

        (Self::new(first_next, second_next), (a_next, b_next))
    }
}

pub fn split<Time: Copy, T, U>(
    first: SF<Time, T>,
    second: SF<Time, U>,
) -> SF<Time, Split<T, U>>
where
    T: SignalTrans<Time>,
    U: SignalTrans<Time>,
{
    SF::from(Split::new(first.into_inner(), second.into_inner()))
}

//
// Fanout
// TODO Rename?
//

// Type alias to simplify some other type declarations
// TODO Add Map type to replace Lift which doesn't require a Time type param
pub type Fanout<Time, T, U> = Compose<
    Lift<
        Time,
        <T as SignalTrans<Time>>::Input,
        (
            <T as SignalTrans<Time>>::Input,
            <T as SignalTrans<Time>>::Input,
        ),
    >,
    Split<T, U>,
>;

pub fn fanout<Time: Copy, T, U>(
    first: SF<Time, T>,
    second: SF<Time, U>,
) -> SF<Time, Fanout<Time, T, U>>
where
    T: SignalTrans<Time>,
    U: SignalTrans<Time, Input = T::Input>,
    T::Input: Clone,
{
    compose(map(|a: T::Input| (a.clone(), a)), split(first, second))
}

//
// Accum
//

#[derive(Clone)]
pub struct Accum<Time, A, B> {
    value: B,
    f: Rc<dyn Fn(Time, A, B) -> B>,
}

impl<Time, A, B> Accum<Time, A, B> {
    pub fn new<F>(init: B, f: F) -> Accum<Time, A, B>
    where
        F: Fn(Time, A, B) -> B + 'static,
    {
        Accum {
            value: init,
            f: Rc::new(f),
        }
    }
}

impl<Time, A, B: Clone> SignalTrans<Time> for Accum<Time, A, B> {
    type Input = A;
    type Output = B;

    fn step(self, delta: Time, a: A) -> (Self, B) {
        let Accum { value, f } = self;
        let value_next = f(delta, a, value);

        (
            Accum {
                value: value_next.clone(),
                f,
            },
            value_next,
        )
    }
}

pub fn accum<Time, A, B, F>(init: B, f: F) -> SF<Time, Accum<Time, A, B>>
where
    B: Clone,
    F: Fn(Time, A, B) -> B + 'static,
{
    SF::from(Accum::new(init, f))
}

pub fn accum_default<Time, A, B, F>(f: F) -> SF<Time, Accum<Time, A, B>>
where
    B: Clone + Default,
    F: Fn(Time, A, B) -> B + 'static,
{
    SF::from(Accum::new(Default::default(), f))
}

//
// Delay
//

#[derive(Copy, Clone, Default)]
pub struct Delay<T> {
    held: T,
}

impl<T> Delay<T> {
    pub fn new(init: T) -> Self {
        Delay { held: init }
    }
}

impl<Time, T> SignalTrans<Time> for Delay<T> {
    type Input = T;
    type Output = T;

    fn step(self, _: Time, value: T) -> (Self, T) {
        let Delay { held: last } = self;
        (Delay { held: value }, last)
    }
}

pub fn delay<Time, A>(init: A) -> SF<Time, Delay<A>> {
    SF::from(Delay::new(init))
}

pub fn delay_default<Time, A: Default>() -> SF<Time, Delay<A>> {
    SF::from(Delay::default())
}
