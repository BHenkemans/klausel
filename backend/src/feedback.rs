use crate::state::Sat;

const FORWARD_LIMIT: usize = 1000;

pub trait Algorithm {
    type State: Clone + PartialEq;
    type Step: Copy;
    type Rule: Copy + PartialEq;

    fn steps(&self, s: &Self::State) -> Vec<Self::Step>;
    fn rule(t: Self::Step) -> Self::Rule;
    fn guard(&self, s: &Self::State, t: Self::Step) -> bool;
    fn applicable(&self, s: &Self::State, t: Self::Step) -> bool;
    fn effect(&self, s: &Self::State, t: Self::Step) -> Option<Self::State>;
    fn result(&self, s: &Self::State) -> Sat;
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ErrorKind {
    Unapplicable,
    Misapplied,
    UnapplicableAndWrong,
}

#[derive(Clone, Debug)]
pub struct StepVerdict<S, R> {
    pub expected: Option<S>,
    pub applicable: bool,
    pub correct: bool,
    pub preempted_by: Option<R>,
    pub finished: Option<Sat>,
}

impl<S, R> StepVerdict<S, R> {
    pub fn valid(&self) -> bool {
        self.applicable && self.correct
    }

    pub fn error(&self) -> Option<ErrorKind> {
        match (self.applicable, self.correct) {
            (true, true) => None,
            (true, false) => Some(ErrorKind::Misapplied),
            (false, true) => Some(ErrorKind::Unapplicable),
            (false, false) => Some(ErrorKind::UnapplicableAndWrong),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TraceVerdict<S, R> {
    pub steps: Vec<StepVerdict<S, R>>,
    pub terminal: bool,
    pub consistent: bool,
    pub correct: bool,
}

impl<S, R> TraceVerdict<S, R> {
    pub fn score(&self) -> (usize, usize) {
        let valid = self.steps.iter().filter(|v| v.valid()).count();
        (valid, self.steps.len())
    }

    pub fn valid(&self) -> bool {
        self.steps.iter().all(StepVerdict::valid) && self.consistent
    }
}

#[derive(Clone, Debug)]
pub struct ForwardRun<S, T> {
    pub steps: Vec<(T, S)>,
    pub result: Sat,
    pub terminated: bool,
}

pub type Verdict<A> = StepVerdict<<A as Algorithm>::State, <A as Algorithm>::Rule>;
pub type Run<A> = ForwardRun<<A as Algorithm>::State, <A as Algorithm>::Step>;

pub fn choice<A: Algorithm>(a: &A, s: &A::State) -> Option<A::Step> {
    a.steps(s).into_iter().find(|&t| a.guard(s, t))
}

pub fn grade_step<A: Algorithm>(
    a: &A,
    from: &A::State,
    t: A::Step,
    claimed: &A::State,
) -> Verdict<A> {
    let expected = a.effect(from, t);
    let applicable = a.applicable(from, t);
    let correct = expected.as_ref() == Some(claimed);
    let chosen = choice(a, from);
    let out_of_order = applicable && correct && !a.guard(from, t);
    StepVerdict {
        preempted_by: chosen.filter(|_| out_of_order).map(A::rule),
        finished: chosen.is_none().then(|| a.result(from)),
        expected,
        applicable,
        correct,
    }
}

pub fn verify<A: Algorithm>(
    a: &A,
    s0: &A::State,
    steps: &[(A::Step, A::State)],
    claimed: Sat,
) -> TraceVerdict<A::State, A::Rule> {
    let verdicts = std::iter::once(s0)
        .chain(steps.iter().map(|(_, s)| s))
        .zip(steps)
        .map(|(from, (t, s))| grade_step(a, from, *t, s))
        .collect();
    let last = steps.last().map_or(s0, |(_, s)| s);
    let terminal = choice(a, last).is_none();
    TraceVerdict {
        steps: verdicts,
        terminal,
        consistent: terminal && a.result(last) == claimed,
        correct: forward(a, s0).result == claimed,
    }
}

pub fn forward<A: Algorithm>(a: &A, s0: &A::State) -> Run<A> {
    let mut s = s0.clone();
    let mut steps = Vec::new();
    for _ in 0..FORWARD_LIMIT {
        let Some(t) = choice(a, &s) else {
            return ForwardRun {
                result: a.result(&s),
                steps,
                terminated: true,
            };
        };
        s = a
            .effect(&s, t)
            .expect("a step whose guard holds has an effect");
        steps.push((t, s.clone()));
    }
    ForwardRun {
        result: a.result(&s),
        steps,
        terminated: false,
    }
}
