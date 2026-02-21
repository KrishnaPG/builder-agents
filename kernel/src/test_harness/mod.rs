// Test harness module
// Property-based testing and COA Simulator

pub mod simulator;

pub use simulator::*;

/// Test harness for running stress tests and certification
pub struct TestHarness;

impl TestHarness {
    /// Run a stress test with the specified parameters
    pub fn run_stress_test(nodes: usize, iterations: usize) -> StressTestReport {
        println!("Running stress test with {} nodes and {} iterations", nodes, iterations);
        
        let config = SimulatorConfig {
            seed: 12345,
            total_operations: iterations as u64,
            max_nodes_per_graph: nodes,
            ..Default::default()
        };
        
        let report = run_simulator(config);
        
        StressTestReport {
            nodes,
            iterations,
            violations: report.violations.len(),
            success: report.passed(),
        }
    }
    
    /// Run certification simulation
    pub fn run_certification() -> CertificationReport {
        println!("Running certification simulation...");
        
        // Run with multiple seeds
        let mut all_passed = true;
        let mut total_violations = 0;
        
        for seed in 0..10 {
            let config = SimulatorConfig {
                seed,
                total_operations: 100_000,
                ..Default::default()
            };
            
            let report = run_simulator(config);
            if !report.passed() {
                all_passed = false;
            }
            total_violations += report.violations.len();
        }
        
        CertificationReport {
            passed: all_passed && total_violations == 0,
            total_violations,
            seeds_tested: 10,
        }
    }
}

/// Report from a stress test
#[derive(Debug, Clone)]
pub struct StressTestReport {
    pub nodes: usize,
    pub iterations: usize,
    pub violations: usize,
    pub success: bool,
}

/// Report from certification
#[derive(Debug, Clone)]
pub struct CertificationReport {
    pub passed: bool,
    pub total_violations: usize,
    pub seeds_tested: u64,
}
