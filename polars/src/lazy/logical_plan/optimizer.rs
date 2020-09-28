use crate::lazy::prelude::*;

pub trait Optimize {
    fn optimize(&self, logical_plan: LogicalPlan) -> LogicalPlan;
}

pub struct ProjectionPushDown {}

impl ProjectionPushDown {
    // We recurrently traverse the logical plan and every projection we encounter we add to the accumulated
    // projections.
    // Every non projection operation we recurse and rebuild that operation on the output of the recursion.
    // The recursion stops at the nodes of the logical plan. These nodes IO or existing DataFrames. On top of
    // these nodes we apply the projection.

    // TODO: renaming operations and joins interfere with the schema. We need to keep track of the schema somehow.

    fn push_down(
        &self,
        logical_plan: LogicalPlan,
        mut accumulated_projections: Vec<Expr>,
    ) -> LogicalPlan {
        use LogicalPlan::*;
        match logical_plan {
            Projection { expr, input } => {
                accumulated_projections.extend(expr);
                self.push_down(*input, accumulated_projections)
            }
            DataFrameScan { df } => LogicalPlanBuilder::from(DataFrameScan { df })
                .project(accumulated_projections)
                .build(),
            CsvScan {
                path,
                schema,
                has_header,
                delimiter,
            } => {
                let lp = CsvScan {
                    path,
                    schema,
                    has_header,
                    delimiter,
                };
                LogicalPlanBuilder::from(lp)
                    .project(accumulated_projections)
                    .build()
            }
            Sort { input, expr } => {
                LogicalPlanBuilder::from(self.push_down(*input, accumulated_projections))
                    .sort(expr)
                    .build()
            }
            Filter { predicate, input } => {
                LogicalPlanBuilder::from(self.push_down(*input, accumulated_projections))
                    .filter(predicate)
                    .build()
            }
        }
    }
}

impl Optimize for ProjectionPushDown {
    fn optimize(&self, logical_plan: LogicalPlan) -> LogicalPlan {
        self.push_down(logical_plan, vec![])
    }
}

#[cfg(test)]
mod test {
    use crate::lazy::logical_plan::optimizer::Optimize;
    use crate::lazy::prelude::*;
    use crate::lazy::tests::get_df;
    use crate::prelude::*;

    #[test]
    fn test_logical_plan() {
        let df = get_df();

        // expensive order
        let lf = df
            .clone()
            .lazy()
            .sort("sepal.width", false)
            .select(&[col("sepal.width")]);

        let logical_plan = lf.logical_plan;
        let opt = ProjectionPushDown {};
        let logical_plan = opt.optimize(logical_plan);
        println!("{}", logical_plan.describe());

        assert!(false)
    }
}