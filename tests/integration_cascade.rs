use std::env;
use std::path::PathBuf;
use std::process::Command;

struct IntegrationCascadeTest {
    manifest_path: PathBuf,
}

impl IntegrationCascadeTest {
    fn from_environment() -> Self {
        Self {
            manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        }
    }

    fn should_run(&self) -> bool {
        env::var_os("ORCHESTRATOR_RUN_GC_INTEGRATION").is_some()
    }

    fn script_path(&self) -> PathBuf {
        env::var("ORCHESTRATOR_TEST_SCRIPT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                self.manifest_path
                    .join("tests")
                    .join("scripts")
                    .join("orchestrator-isolated-gc-test.sh")
            })
    }

    fn orchestrator_binary(&self) -> String {
        env::var("ORCHESTRATOR_BIN")
            .unwrap_or_else(|_| env!("CARGO_BIN_EXE_orchestrator").to_owned())
    }

    fn city_toml(&self) -> String {
        env::var("ORCHESTRATOR_TEST_CITY_TOML").unwrap_or_else(|_| {
            self.manifest_path
                .join("tests")
                .join("fixtures")
                .join("deterministic-city.toml")
                .display()
                .to_string()
        })
    }

    fn run(&self) {
        if !self.should_run() {
            eprintln!("skipping isolated Gas City integration test");
            return;
        }

        let status = Command::new("bash")
            .arg(self.script_path())
            .env("ORCHESTRATOR_BIN", self.orchestrator_binary())
            .env("ORCHESTRATOR_TEST_CITY_TOML", self.city_toml())
            .status()
            .expect("integration script should start");

        assert!(status.success(), "integration script failed: {status}");
    }
}

#[test]
fn isolated_gas_city_cascade_dispatches_end_to_end() {
    IntegrationCascadeTest::from_environment().run();
}
