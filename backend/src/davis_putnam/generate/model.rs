impl Model {
    pub fn from_trace_with_vars(
        trace: &Trace,
        initial_clause_amount: usize,
        var_amount: usize,
    ) -> Self {
        debug_assert!(
            trace
                .iter()
                .map(|step| match step.rule {
                    Rule::Unit | Rule::Pure => step.param / 2,
                    Rule::Elim => step.param,
                })
                .max()
                .is_none_or(|m| m < var_amount),
            "trace references a variable outside var_amount={var_amount}"
        );
        let step_amount = trace.len() + 1;
        Model::new(var_amount, initial_clause_amount, step_amount)
    }
}
