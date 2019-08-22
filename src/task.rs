use num_traits::{Num, Signed};

use crate::{after, map, never, Event, SignalTrans, SF};

pub enum TaskOutput<B, C> {
    Running(B),
    Stopped(C),
}

/*
TODO When/if trait aliases are a thing, use this to shorten some type
constraints. Using the supertrait hack breaks the code since associated types
become type parameters (which leave type parameters unbound)

trait Task<Time, Input, Output, End> =
    SignalTrans<Time, Input = Input, Output = TaskOutput<Output, End>>;
*/

#[derive(Copy, Clone)]
pub enum BasicTask<End, S, T> {
    Running { run: S, stop: T },
    Stopped(End),
}

impl<Time, End, S, T> SignalTrans<Time> for BasicTask<End, S, T>
where
    Time: Copy,
    S: SignalTrans<Time>,
    T: SignalTrans<Time, Input = S::Input, Output = Event<End>>,
    S::Input: Clone,
    End: Clone,
{
    type Input = S::Input;
    type Output = TaskOutput<S::Output, End>;

    fn step(self, delta: Time, input: Self::Input) -> (Self, Self::Output) {
        match self {
            BasicTask::Running { run, stop } => {
                let (stop_next, stop_val) = stop.step(delta, input.clone());
                if let Event::Event(stop_val) = stop_val {
                    (
                        BasicTask::Stopped(stop_val.clone()),
                        TaskOutput::Stopped(stop_val),
                    )
                } else {
                    let (run_next, run_val) = run.step(delta, input);
                    (
                        BasicTask::Running {
                            run: run_next,
                            stop: stop_next,
                        },
                        TaskOutput::Running(run_val),
                    )
                }
            }
            BasicTask::Stopped(val) => (BasicTask::Stopped(val.clone()), TaskOutput::Stopped(val)),
        }
    }
}

pub fn task<Time, End, S, T>(run: SF<Time, S>, stop: SF<Time, T>) -> SF<Time, BasicTask<End, S, T>>
where
    Time: Copy,
    S: SignalTrans<Time>,
    T: SignalTrans<Time, Input = S::Input, Output = Event<End>>,
    S::Input: Clone,
    End: Clone,
{
    SF::from(BasicTask::Running {
        run: run.into_inner(),
        stop: stop.into_inner(),
    })
}

// TODO Write separate struct to avoid some of the type constraints
pub fn forever<Time, Out, End, S>(run: SF<Time, S>) -> SF<Time, impl SignalTrans<Time>>
where
    Time: Copy,
    S: SignalTrans<Time>,
    S::Input: Clone,
    End: Clone,
{
    task(run, never::<Time, S::Input, End>())
}

#[derive(Copy, Clone)]
pub enum AddStop<End, S, T> {
    Running { task: S, stop: T },
    Stopped(End),
}

impl<End, S, T> AddStop<End, S, T> {
    pub fn new(task: S, stop: T) -> Self {
        AddStop::Running {
            task: task,
            stop: stop,
        }
    }
}

impl<Time, Out, End, S, T> SignalTrans<Time> for AddStop<End, S, T>
where
    Time: Copy,
    S: SignalTrans<Time, Output = TaskOutput<Out, End>>,
    T: SignalTrans<Time, Input = S::Input, Output = Event<End>>,
    S::Input: Clone,
    End: Clone,
{
    type Input = S::Input;
    type Output = TaskOutput<Out, End>;

    fn step(self, delta: Time, input: Self::Input) -> (Self, Self::Output) {
        match self {
            AddStop::Running { task, stop } => {
                let (stop_next, stop_val) = stop.step(delta, input.clone());
                if let Event::Event(val) = stop_val {
                    (AddStop::Stopped(val.clone()), TaskOutput::Stopped(val))
                } else {
                    let (task_next, task_val) = task.step(delta, input);
                    match task_val {
                        TaskOutput::Running(val) => {
                            (AddStop::new(task_next, stop_next), TaskOutput::Running(val))
                        }
                        TaskOutput::Stopped(val) => {
                            (AddStop::Stopped(val.clone()), TaskOutput::Stopped(val))
                        }
                    }
                }
            }
            AddStop::Stopped(val) => (AddStop::Stopped(val.clone()), TaskOutput::Stopped(val)),
        }
    }
}

pub fn stop_with<Time, Out, End, S, T>(
    task: SF<Time, S>,
    stop: SF<Time, T>,
) -> SF<Time, AddStop<End, S, T>>
where
    Time: Copy,
    S: SignalTrans<Time, Output = TaskOutput<Out, End>>,
    T: SignalTrans<Time, Input = S::Input, Output = Event<End>>,
    S::Input: Clone,
    End: Clone,
{
    SF::from(AddStop::new(task.into_inner(), stop.into_inner()))
}

pub fn timeout<Time, Out, End, S>(task: SF<Time, S>, time: Time) -> SF<Time, impl SignalTrans<Time>>
where
    Time: Signed + Num + Copy,
    S: SignalTrans<Time, Output = TaskOutput<Out, End>>,
    S::Input: Clone,
    End: Clone,
{
    stop_with(
        task | map(|o| match o {
            TaskOutput::Running(val) => TaskOutput::Running(val),
            TaskOutput::Stopped(val) => TaskOutput::Stopped(Some(val)),
        }),
        after(time, None),
    )
}

// TODO Make an enum storing the state to avoid calling step() on first even
// when it has finished
#[derive(Copy, Clone)]
pub struct SeqTask<S, T> {
    first: S,
    second: T,
}

impl<S, T> SeqTask<S, T> {
    pub fn new(first: S, second: T) -> Self {
        Self {
            first: first,
            second: second,
        }
    }
}

impl<Time, Out, FirstEnd, End, S, T> SignalTrans<Time> for SeqTask<S, T>
where
    Time: Copy,
    S: SignalTrans<Time, Output = TaskOutput<Out, FirstEnd>>,
    T: SignalTrans<Time, Input = S::Input, Output = TaskOutput<Out, End>>,
    S::Input: Clone,
{
    type Input = S::Input;
    type Output = TaskOutput<Out, End>;

    fn step(self, delta: Time, input: Self::Input) -> (Self, Self::Output) {
        let SeqTask { first, second } = self;

        let (first_next, first_val) = first.step(delta, input.clone());
        if let TaskOutput::Running(first_val) = first_val {
            (
                SeqTask::new(first_next, second),
                TaskOutput::Running(first_val),
            )
        } else {
            let (second_next, second_val) = second.step(delta, input);
            (SeqTask::new(first_next, second_next), second_val)
        }
    }
}

pub fn sequence<Time, Out, FirstEnd, End, S, T>(
    first: SF<Time, S>,
    second: SF<Time, T>,
) -> SF<Time, SeqTask<S, T>>
where
    Time: Copy,
    S: SignalTrans<Time, Output = TaskOutput<Out, FirstEnd>>,
    T: SignalTrans<Time, Input = S::Input, Output = TaskOutput<Out, End>>,
    S::Input: Clone,
{
    SF::from(SeqTask::new(first.into_inner(), second.into_inner()))
}

// Makes a binary tree out of the arguments to avoid high nesting of SeqTask
// structs towards the end (or beginning) of the sequence
#[macro_export]
macro_rules! sequence {
    // @level makes a level of the tree by pairing up consecutive elements. e.g.
    // [a, b, c, d, e, f, g] => [(a, b), (c, d), (e, f), g]
    (@level ($a:expr) -> ($($acc:expr),*)) => {
        sequence![$($acc),*, $a]
    };
    (@level ($a:expr, $b:expr) -> ($($acc:expr),*)) => {
        sequence![$($acc,)* sequence($a, $b)]
    };
    (@level ($a:expr, $b:expr, $($rest:tt)*) -> ($($acc:expr),*)) => {
        sequence!(@level ($($rest)*) -> ($($acc,)* sequence($a, $b)))
    };

    [$task:expr] => { $task };
    [$($tasks:expr),+] => {
        sequence!(@level ($($tasks),*) -> ())
    };
    // Allow for trailing comma
    [$($tasks:expr,)+] => {
        sequence!($($tasks),*)
    }
}

pub fn seq_abunch() {
    use crate::*;
    let task1 = task(
        identity::<i32, i32>(),
        map(|i: i32| {
            if i == 1 {
                Event::new(())
            } else {
                Event::NoEvent
            }
        }),
    );
    let task2 = task(
        identity(),
        map(|i: i32| {
            if i == 2 {
                Event::new(i)
            } else {
                Event::NoEvent
            }
        }),
    );

    let seqed = sequence(task1.clone(), task2.clone());
    // let abunch = sequence![seqed, task1, task2.clone(), task2.clone()];
    let working = sequence![
        task1.clone(),
        task2.clone(),
        seqed.clone(),
        task1.clone(),
        task2.clone(),
    ];
}
