use crate::std::marker::PhantomData;

use num_traits::{zero, Num, Signed};

use crate::{accum, SignalTrans, SF};

#[derive(Copy, Clone)]
pub enum Event<T> {
    NoEvent,
    Event(T),
}

impl<T> Event<T> {
    pub fn new(value: T) -> Self {
        Event::Event(value)
    }

    pub fn map<U, F>(self, f: F) -> Event<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Event::Event(value) => Event::new(f(value)),
            Event::NoEvent => Event::NoEvent,
        }
    }

    pub fn filter<F>(self, f: F) -> Self
    where
        F: FnOnce(&T) -> bool,
    {
        match &self {
            Event::Event(value) => {
                if f(value) {
                    self
                } else {
                    Event::NoEvent
                }
            }
            Event::NoEvent => Event::NoEvent,
        }
    }

    pub fn replace<U>(self, new: U) -> Event<U> {
        self.map(move |_| new)
    }
}

impl<A, B> Event<(A, B)> {
    pub fn split(self) -> (Event<A>, Event<B>) {
        match self {
            Event::Event((a, b)) => (Event::new(a), Event::new(b)),
            Event::NoEvent => (Event::NoEvent, Event::NoEvent),
        }
    }
}

impl<T> Default for Event<T> {
    fn default() -> Self {
        Event::NoEvent
    }
}

pub fn fold<Time, A, B: Clone, F>(
    init: B,
    f: F,
) -> SF<Time, impl SignalTrans<Time, Input = Event<A>, Output = B>>
where
    F: Fn(A, B) -> B + 'static,
{
    accum(init, move |_, ev, acc| match ev {
        Event::Event(value) => f(value, acc),
        Event::NoEvent => acc,
    })
}

//
// Never
//

// This could be implemented with constant(), but that requires that the output
// type be Clone
#[derive(Copy, Clone)]
pub struct Never<A, B>(PhantomData<A>, PhantomData<B>);

impl<A, B> Default for Never<A, B> {
    fn default() -> Self {
        Never(Default::default(), Default::default())
    }
}

impl<Time, A, B> SignalTrans<Time> for Never<A, B> {
    type Input = A;
    type Output = Event<B>;

    fn step(self, _: Time, _: A) -> (Self, Event<B>) {
        (self, Event::NoEvent)
    }
}

pub fn never<Time, A, B>() -> SF<Time, Never<A, B>> {
    SF::from(Never::default())
}

//
// After
//

#[derive(Copy, Clone)]
pub enum After<Time, A, B> {
    NotYet {
        time: Time,
        value: B,
        _a: PhantomData<A>,
    },
    Past,
}

impl<Time: Num, A, B: Default> Default for After<Time, A, B> {
    fn default() -> Self {
        Self::new(zero(), B::default())
    }
}

impl<Time, A, B> After<Time, A, B> {
    pub fn new(time: Time, value: B) -> Self {
        After::NotYet {
            time: time,
            value: value,
            _a: Default::default(),
        }
    }
}

impl<Time: Num + Signed, A, B> SignalTrans<Time> for After<Time, A, B> {
    type Input = A;
    type Output = Event<B>;

    fn step(self, delta: Time, _: A) -> (Self, Event<B>) {
        match self {
            After::NotYet { time, value, .. } => {
                let remaining = time - delta;
                if remaining.is_negative() || remaining == zero() {
                    (After::Past, Event::new(value))
                } else {
                    (Self::new(remaining, value), Event::NoEvent)
                }
            }
            After::Past => (After::Past, Event::NoEvent),
        }
    }
}

pub fn after<Time, A, B>(time: Time, value: B) -> SF<Time, After<Time, A, B>>
where
    Time: Num + Signed,
{
    SF::from(After::new(time, value))
}

//
// Edge
//

#[derive(Copy, Clone, Default)]
pub struct Edge {
    last: bool,
}

impl Edge {
    pub fn new(last: bool) -> Self {
        Self { last: last }
    }
}

impl<Time> SignalTrans<Time> for Edge {
    type Input = bool;
    type Output = Event<()>;

    fn step(self, _: Time, on: bool) -> (Self, Event<()>) {
        let ev = if !self.last && on {
            Event::new(())
        } else {
            Event::NoEvent
        };

        (Self::new(on), ev)
    }
}

pub fn edge_init<Time>(init: bool) -> SF<Time, Edge> {
    SF::from(Edge::new(init))
}

pub fn edge<Time>() -> SF<Time, Edge> {
    edge_init(false)
}

//
// Hold
// TODO Implement in terms of fold? (shorter but possibly worse performance)
//

#[derive(Copy, Clone)]
pub struct Hold<A> {
    held: A,
}

impl<A> Hold<A> {
    pub fn new(init: A) -> Self {
        Self { held: init }
    }
}

impl<Time, A: Clone> SignalTrans<Time> for Hold<A> {
    type Input = Event<A>;
    type Output = A;

    fn step(self, _: Time, event: Event<A>) -> (Self, A) {
        match event {
            Event::NoEvent => {
                let value = self.held.clone();
                (self, value)
            }
            Event::Event(value) => (Self::new(value.clone()), value),
        }
    }
}

pub fn hold<Time, A: Clone>(init: A) -> SF<Time, Hold<A>> {
    SF::from(Hold::new(init))
}

//
// Switcher
//

/*
pub struct Switcher {

}

pub fn switcher<A, B, C, T: Clone, U>(init: SF<Time, T>)
-> SF<Time, impl SignalTrans<Time, Input = (A, Event<C>), Output = B>>
where
    T: SignalTrans<Time, Input = A, Output = C>,
    // U: SignalTrans<Time, Input = Event<C>, Output = SF<Time, T>>,
{
    let input = init.clone() | map2(|a, _| a);
    let event = init | map2(|_, e| e);


}
*/
