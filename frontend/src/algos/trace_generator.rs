use rand::Rng;

use backend::state::Sat;

pub trait TraceGenerator: Sized {
    fn len(&self) -> usize;
    fn successors(&self, rng: &mut impl Rng) -> Vec<Vec<Self>>;
    fn closing(&self, target: Sat) -> Option<usize>;
}

fn pick<T>(rng: &mut impl Rng, mut xs: Vec<T>) -> Option<T> {
    (!xs.is_empty()).then(|| xs.swap_remove(rng.random_range(0..xs.len())))
}

pub fn walk<G: TraceGenerator>(start: G, len: usize, target: Sat, rng: &mut impl Rng) -> Option<G> {
    let fits = |g: &G| g.closing(target).is_some_and(|c| g.len() + c <= len);
    let mut run = start;
    while run.len() < len {
        let rules = run
            .successors(rng)
            .into_iter()
            .map(|rule| rule.into_iter().filter(fits).collect::<Vec<_>>())
            .filter(|rule| !rule.is_empty())
            .collect();
        let rule = pick(rng, rules)?;
        run = pick(rng, rule)?;
    }
    (run.closing(target) == Some(0)).then_some(run)
}
