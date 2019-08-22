use crate::std::marker::PhantomData;

pub trait SignalTrans<Time>: Sized {
    type Input;
    type Output;
    // TODO Add Continuation type? Is it really usable in more than a few cases?

    fn step(self, delta: Time, input: Self::Input) -> (Self, Self::Output);
}

// Newtype wrapper so that operators can be overloaded
// TODO Remove if / when rust allows something like
// impl<T: SignalTrans> Trait for T
#[derive(Copy, Clone)]
pub struct SF<Time, S: SignalTrans<Time>> {
    sf: S,
    // TODO Is this really needed since S is constrained using Time?
    _time: PhantomData<Time>,
}

impl<Time, S: SignalTrans<Time>> SF<Time, S> {
    pub fn into_inner(self) -> S {
        self.sf
    }

    pub fn step(self, delta: Time, input: S::Input) -> (Self, S::Output) {
        let (sf, output) = self.sf.step(delta, input);
        (SF::from(sf), output)
    }
}

impl<Time, S: SignalTrans<Time>> From<S> for SF<Time, S> {
    fn from(sf: S) -> Self {
        SF {
            sf: sf,
            _time: Default::default(),
        }
    }
}
