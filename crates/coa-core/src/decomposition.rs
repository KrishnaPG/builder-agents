//! Task decomposition
//!
//! Decomposes high-level specifications into executable tasks.
//! Supports multiple goal types and recursive decomposition.

use crate::error::{DecompositionError, Goal};
use crate::types::{
    AutonomyLevel, DirectiveSet, DirectiveValue, ExpansionType,
    Specification, Task,
};
use coa_composition::StrategySelector;
use coa_symbol::SymbolRefIndex;
use std::str::FromStr;

/// Task decomposer for breaking down specifications
#[derive(Debug)]
pub struct TaskDecomposer {
    strategy_selector: StrategySelector,
    max_depth: usize,
}

impl TaskDecomposer {
    /// Create new task decomposer
    #[inline]
    #[must_use]
    pub fn new(strategy_selector: StrategySelector) -> Self {
        Self {
            strategy_selector,
            max_depth: 5,
        }
    }

    /// With max decomposition depth
    #[inline]
    #[must_use]
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Decompose specification into tasks
    ///
    /// # Arguments
    /// * `spec` - The specification to decompose
    /// * `index` - Symbol index for reference resolution
    ///
    /// # Returns
    /// List of executable tasks
    pub async fn decompose(
        &self,
        spec: Specification,
        _index: &SymbolRefIndex,
    ) -> Result<Vec<Task>, DecompositionError> {
        self.decompose_recursive(spec, 0).await
    }

    /// Recursive decomposition
    async fn decompose_recursive(
        &self,
        spec: Specification,
        depth: usize,
    ) -> Result<Vec<Task>, DecompositionError> {
        if depth > self.max_depth {
            return Err(DecompositionError::RecursionDepthExceeded);
        }

        match spec.goal {
            Goal::CreateNew => self.decompose_create(spec, depth).await,
            Goal::ModifyExisting => self.decompose_modify(spec, depth).await,
            Goal::Refactor => self.decompose_refactor(spec, depth).await,
            Goal::Analyze => self.decompose_analyze(spec, depth).await,
            Goal::Optimize => self.decompose_optimize(spec, depth).await,
        }
    }

    /// Decompose "create new" goal
    async fn decompose_create(
        &self,
        spec: Specification,
        _depth: usize,
    ) -> Result<Vec<Task>, DecompositionError> {
        let mut tasks = Vec::new();

        // 1. Design task
        let design_task = Task::new(
            "architect",
            format!("Design {} structure", spec.artifact_type),
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L3)
        .with_directive("output_format", DirectiveValue::String("design_doc".to_string()));

        tasks.push(design_task);

        // 2. Identify symbols to implement
        let symbols = self.identify_symbols(&spec).await?;

        // 3. Create implementation tasks
        for symbol in &symbols {
            let impl_task = Task::new(
                "implementer",
                format!("Implement {}", symbol),
                spec.target_path.child(symbol),
            )
            .with_autonomy(AutonomyLevel::L4)
            .depends_on(tasks[0].id);

            tasks.push(impl_task);
        }

        // 4. Test generation task
        if !symbols.is_empty() {
            let impl_ids: Vec<_> = tasks.iter().skip(1).map(|t| t.id).collect();
            let test_task = Task::new(
                "tester",
                "Generate tests",
                spec.target_path.child("tests"),
            )
            .with_autonomy(AutonomyLevel::L3)
            .with_directive("coverage_target", DirectiveValue::Int(90));

            // Add dependencies on all implementation tasks
            let test_task = impl_ids.iter().fold(test_task, |task, &id| task.depends_on(id));

            tasks.push(test_task);
        }

        // Apply composition strategy hints
        for task in &mut tasks {
            let strategy_name = self.strategy_selector.select_name(
                &spec.artifact_type,
                &format!("{:?}", spec.goal),
            );
            task.directives.insert(
                "composition_strategy".to_string(),
                DirectiveValue::String(strategy_name.to_string()),
            );
        }

        Ok(tasks)
    }

    /// Decompose "modify existing" goal
    async fn decompose_modify(
        &self,
        spec: Specification,
        _depth: usize,
    ) -> Result<Vec<Task>, DecompositionError> {
        let mut tasks = Vec::new();

        // 1. Analysis task
        let analysis_task = Task::new(
            "analyzer",
            format!("Analyze current {} implementation", spec.artifact_type),
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L3);

        tasks.push(analysis_task);

        // 2. Modification task
        let modify_task = Task::new(
            "modifier",
            format!("Apply modifications to {}", spec.target_path),
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L4)
        .depends_on(tasks[0].id);

        tasks.push(modify_task);

        // 3. Verification task
        let verify_task = Task::new(
            "verifier",
            "Verify modifications",
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L3)
        .depends_on(tasks[1].id);

        tasks.push(verify_task);

        Ok(tasks)
    }

    /// Decompose "refactor" goal
    async fn decompose_refactor(
        &self,
        spec: Specification,
        _depth: usize,
    ) -> Result<Vec<Task>, DecompositionError> {
        let mut tasks = Vec::new();

        // 1. Impact analysis
        let analysis_task = Task::new(
            "analyzer",
            "Analyze refactoring impact",
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L3);

        tasks.push(analysis_task);

        // 2. Create compatibility adapter if needed
        let adapter_task = Task::new(
            "architect",
            "Design compatibility adapter",
            spec.target_path.child("adapter"),
        )
        .with_autonomy(AutonomyLevel::L3)
        .depends_on(tasks[0].id);

        tasks.push(adapter_task);

        // 3. Refactor implementation
        let refactor_task = Task::new(
            "refactorer",
            format!("Refactor {}", spec.target_path),
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L4)
        .depends_on(tasks[1].id);

        tasks.push(refactor_task);

        // 4. Migration/update dependent code
        let migrate_task = Task::new(
            "migrator",
            "Update dependent code",
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L3)
        .with_expansion(ExpansionType::Parallel { branches: vec![] })
        .depends_on(tasks[2].id);

        tasks.push(migrate_task);

        Ok(tasks)
    }

    /// Decompose "analyze" goal
    async fn decompose_analyze(
        &self,
        spec: Specification,
        _depth: usize,
    ) -> Result<Vec<Task>, DecompositionError> {
        let analysis_task = Task::new(
            "analyzer",
            format!("Analyze {}", spec.target_path),
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L3)
        .with_directive("depth", DirectiveValue::String("comprehensive".to_string()));

        Ok(vec![analysis_task])
    }

    /// Decompose "optimize" goal
    async fn decompose_optimize(
        &self,
        spec: Specification,
        _depth: usize,
    ) -> Result<Vec<Task>, DecompositionError> {
        let mut tasks = Vec::new();

        // 1. Benchmark current performance
        let benchmark_task = Task::new(
            "benchmarker",
            "Benchmark current performance",
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L3);

        tasks.push(benchmark_task);

        // 2. Identify optimization opportunities
        let identify_task = Task::new(
            "optimizer",
            "Identify optimization opportunities",
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L3)
        .depends_on(tasks[0].id);

        tasks.push(identify_task);

        // 3. Apply optimizations
        let apply_task = Task::new(
            "optimizer",
            format!("Apply optimizations to {}", spec.target_path),
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L4)
        .depends_on(tasks[1].id);

        tasks.push(apply_task);

        // 4. Verify improvements
        let verify_task = Task::new(
            "benchmarker",
            "Verify performance improvements",
            spec.target_path.clone(),
        )
        .with_autonomy(AutonomyLevel::L3)
        .depends_on(tasks[2].id);

        tasks.push(verify_task);

        Ok(tasks)
    }

    /// Identify symbols to implement from specification
    async fn identify_symbols(
        &self,
        spec: &Specification,
    ) -> Result<Vec<String>, DecompositionError> {
        // In a real implementation, this would:
        // 1. Use LLM to extract symbols from description
        // 2. Parse acceptance criteria for function names
        // 3. Look up existing symbols in index

        // For now, return placeholder based on artifact type
        let symbols = match spec.artifact_type.as_str() {
            "code" => vec!["main".to_string(), "helper".to_string()],
            "config" => vec!["settings".to_string()],
            "spec" => vec!["overview".to_string(), "details".to_string()],
            _ => vec!["item".to_string()],
        };

        Ok(symbols)
    }

    /// Select composition strategy for task
    pub fn select_strategy(&self, spec: &Specification, _task: &Task) -> DirectiveSet {
        let strategy_name = self.strategy_selector.select_name(
            &spec.artifact_type,
            &format!("{:?}", spec.goal),
        );

        let mut directives = DirectiveSet::new();
        directives.insert(
            "composition_strategy".to_string(),
            DirectiveValue::String(strategy_name.to_string()),
        );

        directives
    }
}

impl Default for TaskDecomposer {
    fn default() -> Self {
        Self::new(StrategySelector::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coa_artifact::SymbolPath;

    #[tokio::test]
    async fn decompose_create_goal() {
        let decomposer = TaskDecomposer::default();
        let index = SymbolRefIndex::new();

        let spec = Specification::new(
            Goal::CreateNew,
            "code",
            SymbolPath::from_str("api.auth").unwrap(),
        )
        .with_criteria(vec!["Has login function".to_string()]);

        let tasks = decomposer.decompose(spec, &index).await.unwrap();

        // Should have design + implementations + tests
        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].role, "architect");
    }

    #[tokio::test]
    async fn decompose_modify_goal() {
        let decomposer = TaskDecomposer::default();
        let index = SymbolRefIndex::new();

        let spec = Specification::new(
            Goal::ModifyExisting,
            "code",
            SymbolPath::from_str("api.login").unwrap(),
        );

        let tasks = decomposer.decompose(spec, &index).await.unwrap();

        // Should have analysis + modify + verify
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].role, "analyzer");
        assert_eq!(tasks[1].role, "modifier");
    }

    #[tokio::test]
    async fn decompose_refactor_goal() {
        let decomposer = TaskDecomposer::default();
        let index = SymbolRefIndex::new();

        let spec = Specification::new(
            Goal::Refactor,
            "code",
            SymbolPath::from_str("utils").unwrap(),
        );

        let tasks = decomposer.decompose(spec, &index).await.unwrap();

        // Should have analysis + adapter + refactor + migrate
        assert!(tasks.len() >= 3);
    }

    #[tokio::test]
    async fn decompose_recursion_depth() {
        let decomposer = TaskDecomposer::default().with_max_depth(0);
        let index = SymbolRefIndex::new();

        let spec = Specification::new(
            Goal::CreateNew,
            "code",
            SymbolPath::from_str("test").unwrap(),
        );

        // With max_depth 0, any recursive call should fail
        let _result = decomposer.decompose(spec, &index).await;
    }

    #[test]
    fn task_dependencies() {
        let task1 = Task::new("dev", "task 1", SymbolPath::from_str("a").unwrap());
        let task1_id = task1.id;

        let task2 = Task::new("dev", "task 2", SymbolPath::from_str("b").unwrap())
            .depends_on(task1_id);

        assert_eq!(task2.dependencies.len(), 1);
        assert_eq!(task2.dependencies[0], task1_id);
    }
}
